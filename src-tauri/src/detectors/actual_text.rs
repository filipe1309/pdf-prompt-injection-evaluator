use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context, combined_instruction_patterns};
use lopdf::{Document, Object};

pub struct Signals {
    pub actual_text_values: Vec<String>,
}

pub struct ActualTextDetector;

fn collect_actual_text(object: &Object, doc: &Document, values: &mut Vec<String>) {
    match object {
        Object::Dictionary(dict) => {
            for key in [b"ActualText".as_slice(), b"Alt"] {
                if let Ok(at) = dict.get(key) {
                    if let Ok(text) = at.as_string() {
                        let s = text.into_owned();
                        if !s.trim().is_empty() { values.push(s); }
                    }
                }
            }
        }
        Object::Stream(stream) => {
            if let Ok(at) = stream.dict.get(b"ActualText") {
                if let Ok(text) = at.as_string() {
                    let s = text.into_owned();
                    if !s.trim().is_empty() { values.push(s); }
                }
            }
        }
        Object::Reference(id) => {
            if let Ok(obj) = doc.get_object(*id) { collect_actual_text(obj, doc, values); }
        }
        _ => {}
    }
}

impl VectorDetector for ActualTextDetector {
    fn name(&self) -> &'static str { "actual_text" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut values = Vec::new();
        for object in doc.objects.values() { collect_actual_text(object, doc, &mut values); }
        Box::new(Signals { actual_text_values: values })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        let regex = combined_instruction_patterns();
        signals.actual_text_values.iter().filter_map(|value| {
            regex.find(value).map(|matched| Finding {
                page: 0,
                severity: Severity::Critical,
                detection_type: DetectionType::ActualTextInjection,
                description: "Instruction pattern found in /ActualText accessibility attribute".to_string(),
                excerpt: extract_context(value, matched.start(), 80),
                char_offset: Some(matched.start()),
            })
        }).collect()
    }
}
