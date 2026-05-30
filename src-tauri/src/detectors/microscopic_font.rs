use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::{Document, content::Content};

pub struct Signals {
    pub has_microscopic_font: bool,
}

pub struct MicroscopicFontDetector;

impl VectorDetector for MicroscopicFontDetector {
    fn name(&self) -> &'static str { "microscopic_font" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut has_microscopic_font = false;

        'outer: for (_page_num, page_id) in doc.get_pages() {
            let Ok(content_data) = doc.get_page_content(page_id) else { continue };
            let Ok(content) = Content::decode(&content_data) else { continue };
            let mut font_size: f32 = 12.0;

            for op in &content.operations {
                match op.operator.as_str() {
                    "Tf" => {
                        if op.operands.len() >= 2 {
                            font_size = op.operands[1].as_float().unwrap_or(12.0);
                        }
                    }
                    "Tj" | "TJ" | "'" | "\"" => {
                        if font_size <= 3.0 && font_size > 0.0 {
                            has_microscopic_font = true;
                            break 'outer;
                        }
                    }
                    _ => {}
                }
            }
        }

        Box::new(Signals { has_microscopic_font })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_microscopic_font { return vec![]; }
        vec![Finding {
            page: 0,
            severity: Severity::Warning,
            detection_type: DetectionType::MicroscopicFont,
            description: "Microscopic font size (<=3pt) detected in document".to_string(),
            excerpt: "Text rendered with font size below readable threshold".to_string(),
            char_offset: None,
        }]
    }
}
