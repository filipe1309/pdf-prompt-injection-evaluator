use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context, combined_instruction_patterns};
use lopdf::Document;
use regex::Regex;

pub struct Signals {
    pub ocg_hidden_texts: Vec<String>,
}

pub struct OcgLayerDetector;

fn tj_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\(([^)]*)\)\s*Tj").expect("valid Tj regex"))
}

fn extract_text_from_content_bytes(content: &[u8]) -> String {
    let content_str = String::from_utf8_lossy(content);
    let mut result = String::new();
    for cap in tj_pattern().captures_iter(&content_str) {
        if !result.is_empty() { result.push(' '); }
        let s = cap[1].replace("\\(", "(").replace("\\)", ")").replace("\\\\", "\\");
        result.push_str(&s);
    }
    result
}

fn get_off_ocg_ids(doc: &Document) -> Vec<lopdf::ObjectId> {
    let result: Option<Vec<lopdf::ObjectId>> = (|| {
        let root_obj = doc.trailer.get(b"Root").ok()?;
        let (_, catalog_obj) = doc.dereference(root_obj).ok()?;
        let catalog = catalog_obj.as_dict().ok()?;

        let ocprops_obj = catalog.get(b"OCProperties").ok()?;
        let (_, ocprops_resolved) = doc.dereference(ocprops_obj).ok()?;
        let oc_props = ocprops_resolved.as_dict().ok()?;

        let d_obj = oc_props.get(b"D").ok()?;
        let (_, d_resolved) = doc.dereference(d_obj).ok()?;
        let d_dict = d_resolved.as_dict().ok()?;

        let off_obj = d_dict.get(b"OFF").ok()?;
        let (_, off_resolved) = doc.dereference(off_obj).ok()?;
        let off_arr = off_resolved.as_array().ok()?;

        Some(off_arr.iter().filter_map(|item| item.as_reference().ok()).collect())
    })();
    result.unwrap_or_default()
}

impl VectorDetector for OcgLayerDetector {
    fn name(&self) -> &'static str { "ocg_layer" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let off_ocg_ids = get_off_ocg_ids(doc);

        let mut texts = Vec::new();
        if !off_ocg_ids.is_empty() {
            for (_obj_id, obj) in doc.objects.iter() {
                if let Ok(stream) = obj.as_stream() {
                    if let Ok(oc_ref) = stream.dict.get(b"OC") {
                        if let Ok(oc_id) = oc_ref.as_reference() {
                            if off_ocg_ids.contains(&oc_id) {
                                if let Ok(content) = stream.decompressed_content() {
                                    let text = extract_text_from_content_bytes(&content);
                                    if !text.is_empty() && !texts.contains(&text) {
                                        texts.push(text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Box::new(Signals { ocg_hidden_texts: texts })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        let regex = combined_instruction_patterns();
        signals.ocg_hidden_texts.iter().filter_map(|value| {
            regex.find(value).map(|matched| Finding {
                page: 1,
                severity: Severity::Warning,
                detection_type: DetectionType::HiddenOcgLayer,
                description: "Instruction pattern found in hidden OCG layer content".to_string(),
                excerpt: extract_context(value, matched.start(), 80),
                char_offset: Some(matched.start()),
            })
        }).collect()
    }
}
