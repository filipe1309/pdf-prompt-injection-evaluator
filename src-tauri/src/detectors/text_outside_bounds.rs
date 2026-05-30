use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::{Document, content::Content};

pub struct Signals {
    pub has_text_outside_bounds: bool,
}

pub struct TextOutsideBoundsDetector;

impl VectorDetector for TextOutsideBoundsDetector {
    fn name(&self) -> &'static str { "text_outside_bounds" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut has_text_outside_bounds = false;

        'outer: for (_page_num, page_id) in doc.get_pages() {
            // Get MediaBox from page dictionary
            let (page_width, page_height) = doc.get_dictionary(page_id).ok()
                .and_then(|d| d.get(b"MediaBox").ok().cloned())
                .and_then(|obj| {
                    let resolved = match &obj {
                        lopdf::Object::Reference(id) => doc.get_object(*id).ok().cloned().unwrap_or(obj.clone()),
                        other => other.clone(),
                    };
                    resolved.as_array().ok().cloned()
                })
                .and_then(|arr| {
                    if arr.len() == 4 {
                        let w = arr[2].as_float().ok()?;
                        let h = arr[3].as_float().ok()?;
                        Some((w, h))
                    } else { None }
                })
                .unwrap_or((595.0f32, 842.0f32));

            let Ok(content_data) = doc.get_page_content(page_id) else { continue };
            let Ok(content) = Content::decode(&content_data) else { continue };

            let mut x: f32 = 0.0;
            let mut y: f32 = 0.0;

            for op in &content.operations {
                match op.operator.as_str() {
                    "BT" => { x = 0.0; y = 0.0; }
                    "Td" | "TD" => {
                        if op.operands.len() >= 2 {
                            x += op.operands[0].as_float().unwrap_or(0.0);
                            y += op.operands[1].as_float().unwrap_or(0.0);
                        }
                    }
                    "Tm" => {
                        if op.operands.len() >= 6 {
                            x = op.operands[4].as_float().unwrap_or(0.0);
                            y = op.operands[5].as_float().unwrap_or(0.0);
                        }
                    }
                    "Tj" | "TJ" | "'" | "\"" => {
                        if x < -10.0 || x > page_width + 10.0 || y < -10.0 || y > page_height + 10.0 {
                            has_text_outside_bounds = true;
                            break 'outer;
                        }
                    }
                    _ => {}
                }
            }
        }

        Box::new(Signals { has_text_outside_bounds })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_text_outside_bounds { return vec![]; }
        vec![Finding {
            page: 0,
            severity: Severity::Warning,
            detection_type: DetectionType::TextOutsideBounds,
            description: "Text positioned outside visible page boundaries".to_string(),
            excerpt: "Text coordinates exceed page MediaBox dimensions".to_string(),
            char_offset: None,
        }]
    }
}
