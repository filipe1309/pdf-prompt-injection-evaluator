use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

pub struct Signals {
    pub has_incremental_update: bool,
}

pub struct IncrementalUpdateDetector;

impl VectorDetector for IncrementalUpdateDetector {
    fn name(&self) -> &'static str { "incremental_update" }

    fn extract(&self, _doc: &Document, raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let marker = b"%%EOF";
        let mut count = 0usize;
        let mut pos = 0usize;
        while pos + marker.len() <= raw_bytes.len() {
            if &raw_bytes[pos..pos + marker.len()] == marker {
                count += 1;
                pos += marker.len();
            } else {
                pos += 1;
            }
        }
        Box::new(Signals { has_incremental_update: count > 1 })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_incremental_update { return vec![]; }
        vec![Finding {
            page: 0,
            severity: Severity::Warning,
            detection_type: DetectionType::IncrementalUpdate,
            description: "Incremental update detected — content appended after original PDF structure".to_string(),
            excerpt: "Multiple %%EOF markers found — possible post-signature content injection".to_string(),
            char_offset: None,
        }]
    }
}
