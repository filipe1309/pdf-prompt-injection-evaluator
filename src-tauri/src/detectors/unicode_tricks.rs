use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for UnicodeTricksDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct UnicodeTricksDetectorSignals;

pub struct UnicodeTricksDetector;

impl VectorDetector for UnicodeTricksDetector {
    fn name(&self) -> &'static str { "unicode_tricks" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(UnicodeTricksDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
