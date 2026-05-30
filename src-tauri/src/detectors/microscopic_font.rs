use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

pub struct Signals;
pub struct MicroscopicFontDetector;

impl VectorDetector for MicroscopicFontDetector {
    fn name(&self) -> &'static str { "microscopic_font" }
    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        unimplemented!("MicroscopicFontDetector not yet implemented")
    }
    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        unimplemented!("MicroscopicFontDetector not yet implemented")
    }
}
