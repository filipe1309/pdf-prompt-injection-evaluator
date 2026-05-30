use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

/// Signals extracted from the PDF for MicroscopicFontDetector detection.
/// Replace with a real struct when implementing this detector.
pub struct MicroscopicFontDetectorSignals;

pub struct MicroscopicFontDetector;

impl VectorDetector for MicroscopicFontDetector {
    fn name(&self) -> &'static str { "microscopic_font" }

    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // TODO: implement signal extraction
        Box::new(MicroscopicFontDetectorSignals)
    }

    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        // TODO: implement detection logic
        vec![]
    }
}
