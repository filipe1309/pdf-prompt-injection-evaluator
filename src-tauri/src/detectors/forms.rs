use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context, combined_instruction_patterns};
use lopdf::Document;

pub struct Signals {
    pub has_acroform_fields: bool,
    pub form_field_values: Vec<String>,
}

pub struct FormsDetector;

fn extract_fields(doc: &Document) -> Option<Vec<lopdf::Object>> {
    let root_obj = doc.trailer.get(b"Root").ok()?;
    let (_, catalog_obj) = doc.dereference(root_obj).ok()?;
    let catalog = catalog_obj.as_dict().ok()?;

    let acroform_obj = catalog.get(b"AcroForm").ok()?;
    let (_, acroform_resolved) = doc.dereference(acroform_obj).ok()?;
    let acroform = acroform_resolved.as_dict().ok()?;

    let fields_obj = acroform.get(b"Fields").ok()?;
    let (_, fields_resolved) = doc.dereference(fields_obj).ok()?;
    fields_resolved.as_array().ok().cloned()
}

impl VectorDetector for FormsDetector {
    fn name(&self) -> &'static str { "forms" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut values = Vec::new();

        let fields = match extract_fields(doc) {
            Some(f) => f,
            None => return Box::new(Signals { has_acroform_fields: false, form_field_values: vec![] }),
        };

        for field_ref in &fields {
            let Ok((_, field_obj)) = doc.dereference(field_ref) else { continue };
            let Ok(field_dict) = field_obj.as_dict() else { continue };

            if let Ok(v) = field_dict.get(b"V") {
                if let Ok(text) = v.as_string() { values.push(text.into_owned()); }
            }
        }

        Box::new(Signals { has_acroform_fields: !fields.is_empty(), form_field_values: values })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_acroform_fields { return vec![]; }

        let regex = combined_instruction_patterns();
        let mut findings = vec![Finding {
            page: 0,
            severity: Severity::Warning,
            detection_type: DetectionType::HiddenFormField,
            description: "Hidden form fields (AcroForm) detected in document".to_string(),
            excerpt: "AcroForm fields present — unusual in judicial documents".to_string(),
            char_offset: None,
        }];

        for value in &signals.form_field_values {
            if let Some(matched) = regex.find(value) {
                findings.push(Finding {
                    page: 0,
                    severity: Severity::Critical,
                    detection_type: DetectionType::HiddenFormField,
                    description: "Instruction pattern found in hidden form field value".to_string(),
                    excerpt: extract_context(value, matched.start(), 80),
                    char_offset: Some(matched.start()),
                });
            }
        }
        findings
    }
}
