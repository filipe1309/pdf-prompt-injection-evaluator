use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for InvisibleTextDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct InvisibleTextDetectorSignals;

pub struct InvisibleTextDetector;

impl VectorDetector for InvisibleTextDetector {
    fn name(&self) -> &'static str { "invisible_text" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(InvisibleTextDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
