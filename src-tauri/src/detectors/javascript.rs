use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::{Document, Object};

pub struct Signals {
    pub javascript_code: Option<String>,
}

pub struct JavaScriptDetector;

fn extract_js_string(obj: &Object, doc: &Document) -> Option<String> {
    match obj {
        Object::String(bytes, _) => {
            let s = String::from_utf8_lossy(bytes).to_string();
            if !s.is_empty() { Some(s) } else { None }
        }
        Object::Stream(stream) => {
            stream.decompressed_content().ok().and_then(|bytes| {
                let s = String::from_utf8_lossy(&bytes).to_string();
                if !s.is_empty() { Some(s) } else { None }
            })
        }
        Object::Reference(id) => {
            if let Ok(resolved) = doc.get_object(*id) { extract_js_string(resolved, doc) } else { None }
        }
        _ => None,
    }
}

fn extract_js_from_object(object: &Object, doc: &Document, depth: u8) -> Option<String> {
    if depth > 5 { return None; }
    match object {
        Object::Dictionary(dict) => {
            for key in [b"JS".as_slice(), b"JavaScript"] {
                if let Ok(js_obj) = dict.get(key) {
                    if let Some(js) = extract_js_string(js_obj, doc) { return Some(js); }
                }
            }
            None
        }
        Object::Stream(stream) => {
            for key in [b"JS".as_slice(), b"JavaScript"] {
                if let Ok(js_obj) = stream.dict.get(key) {
                    if let Some(js) = extract_js_string(js_obj, doc) { return Some(js); }
                }
            }
            None
        }
        Object::Reference(id) => {
            if let Ok(obj) = doc.get_object(*id) { extract_js_from_object(obj, doc, depth + 1) } else { None }
        }
        _ => None,
    }
}

impl VectorDetector for JavaScriptDetector {
    fn name(&self) -> &'static str { "javascript" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let javascript_code = doc.objects.iter()
            .find_map(|(_, obj)| extract_js_from_object(obj, doc, 0));
        Box::new(Signals { javascript_code })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        let Some(ref js_code) = signals.javascript_code else { return vec![]; };
        let excerpt = if js_code.len() > 80 { format!("{}...", &js_code[..80]) } else { js_code.clone() };
        vec![Finding {
            page: 0,
            severity: Severity::Critical,
            detection_type: DetectionType::EmbeddedJavaScript,
            description: "Embedded JavaScript detected in PDF document".to_string(),
            excerpt,
            char_offset: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::detectors::SharedSignals;

    #[test]
    fn no_js_no_findings() {
        let signals = Signals { javascript_code: None };
        let d = JavaScriptDetector;
        let findings = d.detect(&signals, &HashMap::new(), &SharedSignals::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn js_produces_critical_finding() {
        let signals = Signals { javascript_code: Some("app.alert('test')".to_string()) };
        let d = JavaScriptDetector;
        let findings = d.detect(&signals, &HashMap::new(), &SharedSignals::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].detection_type, crate::models::DetectionType::EmbeddedJavaScript);
        assert_eq!(findings[0].severity, crate::models::Severity::Critical);
    }
}
