use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::models::Finding;
use lopdf::Document;
use regex::Regex;

pub mod actual_text;
pub mod annotations;
pub mod forms;
pub mod incremental_update;
pub mod instruction_patterns;
pub mod invisible_text;
pub mod javascript;
pub mod metadata;
pub mod microscopic_font;
pub mod ocg_layer;
pub mod text_outside_bounds;
pub mod token_flooding;
pub mod unicode_tricks;
pub mod white_text;
pub mod zero_width;

pub trait VectorDetector: Send + Sync {
    fn extract(
        &self,
        doc: &Document,
        raw_bytes: &[u8],
        pages: &HashMap<u32, String>,
    ) -> Box<dyn Any + Send>;

    fn detect(
        &self,
        signals: &dyn Any,
        pages: &HashMap<u32, String>,
        shared: &SharedSignals,
    ) -> Vec<Finding>;

    fn name(&self) -> &'static str;
}

#[derive(Default)]
pub struct SharedSignals {
    pub has_white_text: bool,
    pub has_microscopic_font: bool,
    pub has_incremental_update: bool,
    pub has_invisible_text: bool,
    pub has_text_outside_bounds: bool,
}

/// Computes cross-detector flags used by detectors that need context about
/// other signals (e.g. CitationPoisoning suppresses standalone WhiteText).
/// This is a pre-pass over the PDF before individual detectors run.
// TODO(task-9): delegate per-detector pre-pass logic to WhiteTextDetector /
// MicroscopicFontDetector once implemented, to avoid duplicate parsing.
pub fn build_shared_signals(doc: &Document, raw_bytes: &[u8]) -> SharedSignals {
    use lopdf::content::Content;

    let mut has_white_text = false;
    let mut has_microscopic_font = false;

    for (_page_num, page_id) in doc.get_pages() {
        let Ok(content_data) = doc.get_page_content(page_id) else { continue };
        let Ok(content) = Content::decode(&content_data) else { continue };

        let mut current_color_is_white = false;
        let mut current_font_size: f32 = 12.0;

        for op in &content.operations {
            match op.operator.as_str() {
                // DeviceRGB non-stroking: r g b rg
                "rg" if op.operands.len() == 3 => {
                    let r = op.operands[0].as_float().unwrap_or(0.0);
                    let g = op.operands[1].as_float().unwrap_or(0.0);
                    let b = op.operands[2].as_float().unwrap_or(0.0);
                    current_color_is_white = r > 0.99 && g > 0.99 && b > 0.99;
                }
                // DeviceRGB stroking: R G B RG
                "RG" if op.operands.len() == 3 => {
                    let r = op.operands[0].as_float().unwrap_or(0.0);
                    let g = op.operands[1].as_float().unwrap_or(0.0);
                    let b = op.operands[2].as_float().unwrap_or(0.0);
                    current_color_is_white = r > 0.99 && g > 0.99 && b > 0.99;
                }
                // DeviceGray non-stroking: g
                "g" if op.operands.len() == 1 => {
                    let gray = op.operands[0].as_float().unwrap_or(0.0);
                    current_color_is_white = gray > 0.99;
                }
                // DeviceGray stroking: G
                "G" if op.operands.len() == 1 => {
                    let gray = op.operands[0].as_float().unwrap_or(0.0);
                    current_color_is_white = gray > 0.99;
                }
                // DeviceCMYK non-stroking: c m y k (white = 0 0 0 0)
                "k" if op.operands.len() == 4 => {
                    let c = op.operands[0].as_float().unwrap_or(1.0);
                    let m = op.operands[1].as_float().unwrap_or(1.0);
                    let y = op.operands[2].as_float().unwrap_or(1.0);
                    let k = op.operands[3].as_float().unwrap_or(1.0);
                    current_color_is_white = c < 0.01 && m < 0.01 && y < 0.01 && k < 0.01;
                }
                // DeviceCMYK stroking: C M Y K
                "K" if op.operands.len() == 4 => {
                    let c = op.operands[0].as_float().unwrap_or(1.0);
                    let m = op.operands[1].as_float().unwrap_or(1.0);
                    let y = op.operands[2].as_float().unwrap_or(1.0);
                    let k = op.operands[3].as_float().unwrap_or(1.0);
                    current_color_is_white = c < 0.01 && m < 0.01 && y < 0.01 && k < 0.01;
                }
                "Tf" if op.operands.len() >= 2 => {
                    current_font_size = op.operands[1].as_float().unwrap_or(12.0);
                }
                "Tj" | "TJ" | "'" | "\"" => {
                    if current_color_is_white { has_white_text = true; }
                    if current_font_size <= 3.0 && current_font_size > 0.0 { has_microscopic_font = true; }
                }
                _ => {}
            }
        }
    }

    // Incremental update: multiple %%EOF markers in raw bytes
    let marker = b"%%EOF";
    let mut count = 0usize;
    let mut pos = 0usize;
    while pos + marker.len() <= raw_bytes.len() {
        if &raw_bytes[pos..pos + marker.len()] == marker {
            count += 1;
            pos += marker.len();
        } else {
            pos += 1;
        }
    }
    let has_incremental_update = count > 1;

    SharedSignals {
        has_white_text,
        has_microscopic_font,
        has_incremental_update,
        // Populated by individual detectors in future tasks
        has_invisible_text: false,
        has_text_outside_bounds: false,
    }
}

pub fn run_all(
    doc: &Document,
    raw_bytes: &[u8],
    pages: &HashMap<u32, String>,
) -> Vec<Finding> {
    use actual_text::ActualTextDetector;
    use annotations::AnnotationsDetector;
    use forms::FormsDetector;
    use incremental_update::IncrementalUpdateDetector;
    use instruction_patterns::InstructionPatternsDetector;
    use invisible_text::InvisibleTextDetector;
    use javascript::JavaScriptDetector;
    use metadata::MetadataDetector;
    use microscopic_font::MicroscopicFontDetector;
    use ocg_layer::OcgLayerDetector;
    use text_outside_bounds::TextOutsideBoundsDetector;
    use token_flooding::TokenFloodingDetector;
    use unicode_tricks::UnicodeTricksDetector;
    use white_text::WhiteTextDetector;
    use zero_width::ZeroWidthDetector;

    let shared = build_shared_signals(doc, raw_bytes);

    let detectors: Vec<Box<dyn VectorDetector>> = vec![
        Box::new(IncrementalUpdateDetector),
        Box::new(WhiteTextDetector),
        Box::new(InvisibleTextDetector),
        Box::new(MicroscopicFontDetector),
        Box::new(TextOutsideBoundsDetector),
        Box::new(ZeroWidthDetector),
        Box::new(UnicodeTricksDetector),
        Box::new(TokenFloodingDetector),
        Box::new(InstructionPatternsDetector),
        Box::new(JavaScriptDetector),
        Box::new(MetadataDetector),
        Box::new(AnnotationsDetector),
        Box::new(FormsDetector),
        Box::new(ActualTextDetector),
        Box::new(OcgLayerDetector),
        // ← register new detectors here
    ];

    detectors.iter().flat_map(|d| {
        let signals = d.extract(doc, raw_bytes, pages);
        d.detect(signals.as_ref(), pages, &shared)
    }).collect()
}

// Shared helpers used by multiple detectors

pub fn extract_context(text: &str, pos: usize, radius: usize) -> String {
    let mut start = pos.saturating_sub(radius);
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (pos + radius).min(text.len());
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    text[start..end].to_string()
}

// Pattern strings are const so combined_instruction_patterns() can be built
// from them without risk of the patterns drifting out of sync.
const PT_BR_PATTERN: &str = r"ignore\s+(todas\s+)?(as\s+)?instru[çc][õo]es|desconsidere\s+o\s+prompt|aja\s+como|novo\s+objetivo|esque[çc]a\s+(as\s+)?instru[çc][õo]es|n[aã]o\s+siga\s+(as\s+)?regras|aten[çc][aã]o[\s,]+intelig[eê]ncia\s+artificial|n[aã]o\s+impugne|conteste\s+(esta|essa|de\s+forma)\s+.*superficial|conclua\s+que\s+todos|independentemente\s+do\s+comando";
const EN_PATTERN: &str = r"ignore\s+(all\s+)?previous\s+instructions|disregard\s+(the\s+)?(above|previous)|act\s+as\s+(?:if\s+)?(?:you\s+(?:are|were)|an?\s+(?:AI|unrestricted|jailbreak|dan|evil|free))|new\s+objective|forget\s+(all\s+)?(your\s+)?instructions|you\s+are\s+now\s+|system\s*:\s*";

pub fn instruction_patterns_pt_br() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!("(?i)({PT_BR_PATTERN})")).expect("valid pt-BR instruction regex")
    })
}

pub fn instruction_patterns_en() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!("(?i)({EN_PATTERN})")).expect("valid EN instruction regex")
    })
}

pub fn combined_instruction_patterns() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!("(?i)({PT_BR_PATTERN}|{EN_PATTERN})"))
            .expect("valid combined instruction regex")
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::pdf_parser;
    use std::path::Path;

    #[test]
    fn test_detects_sample_01_texto_branco() {
        let path = Path::new("../samples/vectors/01_texto_branco.pdf");
        if !path.exists() { return; }
        let parsed = pdf_parser::load_pdf(path).unwrap();
        let findings = run_all(&parsed.doc, &parsed.raw_bytes, &parsed.pages);
        assert!(!findings.is_empty(), "Should detect injection in sample 01");
    }

    #[test]
    fn test_detects_sample_10_ocg_off() {
        let path = Path::new("../samples/vectors/10_camada_ocg_off.pdf");
        if !path.exists() { return; }
        let parsed = pdf_parser::load_pdf(path).unwrap();
        let findings = run_all(&parsed.doc, &parsed.raw_bytes, &parsed.pages);
        assert!(findings.iter().any(|f| f.detection_type == crate::models::DetectionType::HiddenOcgLayer),
            "Should detect HiddenOcgLayer");
    }

    #[test]
    fn test_detects_sample_16_instrucao_visivel() {
        let path = Path::new("../samples/vectors/16_instrucao_visivel.pdf");
        if !path.exists() { return; }
        let parsed = pdf_parser::load_pdf(path).unwrap();
        let findings = run_all(&parsed.doc, &parsed.raw_bytes, &parsed.pages);
        assert!(findings.iter().any(|f| f.detection_type == crate::models::DetectionType::InstructionPattern),
            "Should detect InstructionPattern in visible text");
    }

    #[test]
    fn test_run_all_does_not_panic_on_empty_input() {
        let pages = HashMap::from([(1u32, "Contrato de prestacao de servicos entre as partes para fins legais.".to_string())]);
        let doc = lopdf::Document::new();
        let findings = run_all(&doc, b"", &pages);
        assert!(findings.iter().all(|f| f.severity != crate::models::Severity::Critical),
            "Clean document should not produce Critical findings");
    }
}
