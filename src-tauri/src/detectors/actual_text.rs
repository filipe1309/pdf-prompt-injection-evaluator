use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for ActualTextDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct ActualTextDetectorSignals;

pub struct ActualTextDetector;

impl VectorDetector for ActualTextDetector {
    fn name(&self) -> &'static str { "actual_text" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(ActualTextDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
