use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::{Document, content::Content};

pub struct Signals {
    pub has_invisible_text: bool,
}

pub struct InvisibleTextDetector;

impl VectorDetector for InvisibleTextDetector {
    fn name(&self) -> &'static str { "invisible_text" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut has_invisible_text = false;

        'outer: for (_page_num, page_id) in doc.get_pages() {
            let Ok(content_data) = doc.get_page_content(page_id) else { continue };
            let Ok(content) = Content::decode(&content_data) else { continue };
            let mut render_mode: i32 = 0;

            for op in &content.operations {
                match op.operator.as_str() {
                    "BT" => { render_mode = 0; }
                    "Tr" => {
                        if !op.operands.is_empty() {
                            render_mode = op.operands[0].as_i64().unwrap_or(0) as i32;
                        }
                    }
                    "Tj" | "TJ" | "'" | "\"" => {
                        if render_mode == 3 {
                            has_invisible_text = true;
                            break 'outer;
                        }
                    }
                    _ => {}
                }
            }
        }

        Box::new(Signals { has_invisible_text })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_invisible_text || shared.has_incremental_update { return vec![]; }
        vec![Finding {
            page: 0,
            severity: Severity::Warning,
            detection_type: DetectionType::InvisibleText,
            description: "Invisible text (render mode 3) detected in document".to_string(),
            excerpt: "Text present in extraction layer but not rendered visually (Tr 3)".to_string(),
            char_offset: None,
        }]
    }
}
