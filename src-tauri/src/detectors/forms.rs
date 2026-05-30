use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

pub struct Signals;
pub struct FormsDetector;

impl VectorDetector for FormsDetector {
    fn name(&self) -> &'static str { "forms" }
    fn extract(&self, _doc: &Document, _raw: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        unimplemented!("FormsDetector not yet implemented")
    }
    fn detect(&self, _s: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        unimplemented!("FormsDetector not yet implemented")
    }
}
