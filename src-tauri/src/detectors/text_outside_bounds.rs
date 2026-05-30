use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for TextOutsideBoundsDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct TextOutsideBoundsDetectorSignals;

pub struct TextOutsideBoundsDetector;

impl VectorDetector for TextOutsideBoundsDetector {
    fn name(&self) -> &'static str { "text_outside_bounds" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(TextOutsideBoundsDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
