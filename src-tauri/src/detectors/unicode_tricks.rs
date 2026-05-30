use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context};
use lopdf::Document;

pub struct Signals {
    pub bidi_positions: Vec<(u32, usize)>, // (page, char_offset)
}

pub struct UnicodeTricksDetector;

impl VectorDetector for UnicodeTricksDetector {
    fn name(&self) -> &'static str { "unicode_tricks" }

    fn extract(&self, _doc: &Document, _raw_bytes: &[u8], pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut positions = Vec::new();
        for (page, text) in pages {
            for (pos, ch) in text.char_indices() {
                let is_bidi = ('\u{202A}'..='\u{202E}').contains(&ch)
                    || ('\u{2066}'..='\u{2069}').contains(&ch);
                if is_bidi { positions.push((*page, pos)); }
            }
        }
        Box::new(Signals { bidi_positions: positions })
    }

    fn detect(&self, signals: &dyn Any, pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        signals.bidi_positions.iter().map(|(page, pos)| {
            let text = pages.get(page).map(String::as_str).unwrap_or("");
            Finding {
                page: *page,
                severity: Severity::Critical,
                detection_type: DetectionType::UnicodeTrick,
                description: "Bidirectional Unicode override character detected".to_string(),
                excerpt: extract_context(text, *pos, 30),
                char_offset: Some(*pos),
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::detectors::SharedSignals;

    #[test]
    fn detects_bidi_override() {
        let text = format!("Visible {} hidden", '\u{202E}');
        let pages = HashMap::from([(7u32, text.clone())]);
        let d = UnicodeTricksDetector;
        let signals = d.extract(&lopdf::Document::new(), &[], &pages);
        let findings = d.detect(signals.as_ref(), &pages, &SharedSignals::default());
        assert!(!findings.is_empty());
        assert_eq!(findings[0].detection_type, crate::models::DetectionType::UnicodeTrick);
        assert_eq!(findings[0].severity, crate::models::Severity::Critical);
        assert_eq!(findings[0].page, 7);
    }

    #[test]
    fn clean_text_no_findings() {
        let pages = HashMap::from([(1u32, "Normal text".to_string())]);
        let d = UnicodeTricksDetector;
        let signals = d.extract(&lopdf::Document::new(), &[], &pages);
        let findings = d.detect(signals.as_ref(), &pages, &SharedSignals::default());
        assert!(findings.is_empty());
    }
}
