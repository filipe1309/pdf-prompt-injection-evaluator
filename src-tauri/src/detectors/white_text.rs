use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals, extract_context};
use lopdf::{Document, content::Content};
use regex::Regex;

pub struct Signals {
    pub has_white_text: bool,
    pub white_text_pages: Vec<u32>,
}

pub struct WhiteTextDetector;

fn fake_citation_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(s[uú]mula\s+([5-9]\d{2}|\d{4,})/(TST|STF|STJ)|OJ-SDI\d?-([5-9]\d{2}|\d{4,})|PRECEDENTE\s+VINCULANTE)"
        ).expect("valid fake citation regex")
    })
}

impl VectorDetector for WhiteTextDetector {
    fn name(&self) -> &'static str { "white_text" }

    fn extract(&self, doc: &Document, _raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        let mut has_white_text = false;
        let mut white_text_pages = Vec::new();

        for (page_num, page_id) in doc.get_pages() {
            let Ok(content_data) = doc.get_page_content(page_id) else { continue };
            let Ok(content) = Content::decode(&content_data) else { continue };
            let mut color_is_white = false;

            for op in &content.operations {
                match op.operator.as_str() {
                    "rg" | "RG" if op.operands.len() == 3 => {
                        let r = op.operands[0].as_float().unwrap_or(0.0);
                        let g = op.operands[1].as_float().unwrap_or(0.0);
                        let b = op.operands[2].as_float().unwrap_or(0.0);
                        color_is_white = r > 0.99 && g > 0.99 && b > 0.99;
                    }
                    "g" | "G" if op.operands.len() == 1 => {
                        color_is_white = op.operands[0].as_float().unwrap_or(0.0) > 0.99;
                    }
                    "k" | "K" if op.operands.len() == 4 => {
                        let c = op.operands[0].as_float().unwrap_or(1.0);
                        let m = op.operands[1].as_float().unwrap_or(1.0);
                        let y = op.operands[2].as_float().unwrap_or(1.0);
                        let k = op.operands[3].as_float().unwrap_or(1.0);
                        color_is_white = c < 0.01 && m < 0.01 && y < 0.01 && k < 0.01;
                    }
                    "Tj" | "TJ" | "'" | "\"" if color_is_white => {
                        has_white_text = true;
                        if !white_text_pages.contains(&page_num) {
                            white_text_pages.push(page_num);
                        }
                    }
                    _ => {}
                }
            }
        }

        Box::new(Signals { has_white_text, white_text_pages })
    }

    fn detect(&self, signals: &dyn Any, pages: &HashMap<u32, String>, _shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        if !signals.has_white_text { return vec![]; }

        let fake_citation = fake_citation_regex();
        let mut findings = Vec::new();
        let mut citation_found = false;

        // Check pages with white text for citation poisoning
        for page in &signals.white_text_pages {
            if let Some(text) = pages.get(page) {
                if let Some(matched) = fake_citation.find(text) {
                    citation_found = true;
                    findings.push(Finding {
                        page: *page,
                        severity: Severity::Warning,
                        detection_type: DetectionType::CitationPoisoning,
                        description: "Fabricated legal citation detected — fake jurisprudence to mislead AI analysis".to_string(),
                        excerpt: extract_context(text, matched.start(), 80),
                        char_offset: Some(matched.start()),
                    });
                }
            }
        }

        // If no citation found on white-text pages, scan all pages
        if !citation_found {
            for (page, text) in pages {
                if let Some(matched) = fake_citation.find(text) {
                    citation_found = true;
                    findings.push(Finding {
                        page: *page,
                        severity: Severity::Warning,
                        detection_type: DetectionType::CitationPoisoning,
                        description: "Fabricated legal citation detected — fake jurisprudence to mislead AI analysis".to_string(),
                        excerpt: extract_context(text, matched.start(), 80),
                        char_offset: Some(matched.start()),
                    });
                }
            }
        }

        // Only emit standalone WhiteText if no citation poisoning found
        if !citation_found {
            findings.push(Finding {
                page: 0,
                severity: Severity::Warning,
                detection_type: DetectionType::WhiteText,
                description: "White/invisible text detected in document content stream".to_string(),
                excerpt: "Color set to white (1,1,1) before text rendering".to_string(),
                char_offset: None,
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

    #[test]
    fn no_white_text_signals_no_findings() {
        let shared = SharedSignals::default();
        let signals = Signals { has_white_text: false, white_text_pages: vec![] };
        let d = WhiteTextDetector;
        let findings = d.detect(&signals, &HashMap::new(), &shared);
        assert!(findings.is_empty());
    }

    #[test]
    fn white_text_without_citation_produces_finding() {
        let shared = SharedSignals { has_white_text: true, ..SharedSignals::default() };
        let signals = Signals { has_white_text: true, white_text_pages: vec![] };
        let d = WhiteTextDetector;
        let findings = d.detect(&signals, &HashMap::new(), &shared);
        assert!(findings.iter().any(|f| f.detection_type == crate::models::DetectionType::WhiteText));
    }

    #[test]
    fn citation_poisoning_suppresses_standalone_white_text() {
        let shared = SharedSignals { has_white_text: true, ..SharedSignals::default() };
        let pages = HashMap::from([(1u32, "Súmula 950/STF determina que".to_string())]);
        let signals = Signals { has_white_text: true, white_text_pages: vec![1] };
        let d = WhiteTextDetector;
        let findings = d.detect(&signals, &pages, &shared);
        assert!(findings.iter().any(|f| f.detection_type == crate::models::DetectionType::CitationPoisoning));
        assert!(!findings.iter().any(|f| f.detection_type == crate::models::DetectionType::WhiteText));
    }
}
