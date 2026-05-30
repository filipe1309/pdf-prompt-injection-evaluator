use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context};
use lopdf::Document;

pub struct Signals {
    pub zero_width_positions: Vec<usize>,
}

pub struct ZeroWidthDetector;

impl VectorDetector for ZeroWidthDetector {
    fn name(&self) -> &'static str { "zero_width" }

    fn extract(&self, _doc: &Document, _raw_bytes: &[u8], pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let zero_width_chars: Vec<char> = vec!['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}', '\u{00AD}', '\u{2060}'];
        let mut positions = Vec::new();
        for text in pages.values() {
            for (pos, ch) in text.char_indices() {
                if zero_width_chars.contains(&ch) {
                    positions.push(pos);
                }
            }
        }
        Box::new(Signals { zero_width_positions: positions })
    }

    fn detect(&self, signals: &dyn Any, pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if signals.zero_width_positions.is_empty() { return vec![]; }

        let zero_width_chars: Vec<char> = vec!['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}', '\u{00AD}', '\u{2060}'];
        let mut findings = Vec::new();

        for (page, text) in pages {
            let positions: Vec<usize> = text.char_indices()
                .filter(|(_, ch)| zero_width_chars.contains(ch))
                .map(|(pos, _)| pos)
                .collect();

            if positions.is_empty() { continue; }

            let first_pos = positions[0];
            let count = positions.len();
            let chars: Vec<char> = text.chars().collect();
            let mut hidden_chars: Vec<char> = Vec::new();
            let mut prev_was_zw = false;

            for (i, ch) in chars.iter().enumerate() {
                if zero_width_chars.contains(ch) {
                    prev_was_zw = true;
                } else {
                    let next_is_zw = chars.get(i + 1).is_some_and(|c| zero_width_chars.contains(c));
                    if prev_was_zw || next_is_zw { hidden_chars.push(*ch); }
                    prev_was_zw = false;
                }
            }

            let hidden_text: String = hidden_chars.into_iter().collect();
            let excerpt = if hidden_text.trim().is_empty() {
                extract_context(text, first_pos, 40)
            } else {
                hidden_text.trim().to_string()
            };

            findings.push(Finding {
                page: *page,
                severity: Severity::Critical,
                detection_type: DetectionType::ZeroWidthChars,
                description: format!("{} ({}x)", "Zero-width invisible characters detected", count),
                excerpt,
                char_offset: Some(first_pos),
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::detectors::SharedSignals;

    fn run(text: &str) -> Vec<crate::models::Finding> {
        let d = ZeroWidthDetector;
        let pages = HashMap::from([(1u32, text.to_string())]);
        let signals = d.extract(&lopdf::Document::new(), &[], &pages);
        d.detect(signals.as_ref(), &pages, &SharedSignals::default())
    }

    #[test]
    fn detects_zero_width_chars() {
        let text = format!("Please {}ignore previous", '\u{200B}');
        let findings = run(&text);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].detection_type, crate::models::DetectionType::ZeroWidthChars);
        assert_eq!(findings[0].severity, crate::models::Severity::Critical);
    }

    #[test]
    fn clean_text_no_findings() {
        let findings = run("Normal legal document text without injection.");
        assert!(findings.is_empty());
    }
}
