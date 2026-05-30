use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for IncrementalUpdateDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct IncrementalUpdateDetectorSignals;

pub struct IncrementalUpdateDetector;

impl VectorDetector for IncrementalUpdateDetector {
    fn name(&self) -> &'static str { "incremental_update" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(IncrementalUpdateDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
