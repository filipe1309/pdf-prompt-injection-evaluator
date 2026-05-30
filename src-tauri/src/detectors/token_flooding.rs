use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

const FLOODING_TERMS: &[&str] = &[
    "procedente", "dano moral", "ma-fe", "confissao", "revelia",
    "presuncao de veracidade", "incontroverso", "prova irrefutavel",
];
const FLOODING_THRESHOLD: usize = 10;

pub struct Signals {
    pub hits_per_page: Vec<(u32, usize)>,
}

pub struct TokenFloodingDetector;

impl VectorDetector for TokenFloodingDetector {
    fn name(&self) -> &'static str { "token_flooding" }

    fn extract(&self, _doc: &Document, _raw_bytes: &[u8], pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let hits_per_page = pages.iter().map(|(page, text)| {
            let lower = text.to_lowercase();
            let total: usize = FLOODING_TERMS.iter().map(|t| lower.matches(t).count()).sum();
            (*page, total)
        }).collect();
        Box::new(Signals { hits_per_page })
    }

    fn detect(&self, signals: &dyn Any, _pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        signals.hits_per_page.iter()
            .filter(|(_, total)| *total > FLOODING_THRESHOLD)
            .map(|(page, total)| Finding {
                page: *page,
                severity: Severity::Warning,
                detection_type: DetectionType::TokenFlooding,
                description: "Token flooding detected: excessive repetition of favorable legal terms".to_string(),
                excerpt: format!("{} repetitions of favorable terms detected", total),
                char_offset: None,
            }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::detectors::SharedSignals;

    #[test]
    fn detects_flooding() {
        let repeated = "procedente ".repeat(15);
        let pages = HashMap::from([(1u32, repeated)]);
        let d = TokenFloodingDetector;
        let signals = d.extract(&lopdf::Document::new(), &[], &pages);
        let findings = d.detect(signals.as_ref(), &pages, &SharedSignals::default());
        assert!(!findings.is_empty());
        assert_eq!(findings[0].detection_type, crate::models::DetectionType::TokenFlooding);
    }

    #[test]
    fn clean_text_no_flooding() {
        let pages = HashMap::from([(1u32, "procedente once".to_string())]);
        let d = TokenFloodingDetector;
        let signals = d.extract(&lopdf::Document::new(), &[], &pages);
        let findings = d.detect(signals.as_ref(), &pages, &SharedSignals::default());
        assert!(findings.is_empty());
    }
}
