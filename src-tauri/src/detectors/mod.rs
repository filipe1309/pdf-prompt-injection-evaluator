use std::any::Any;
use std::collections::HashMap;
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
}

pub fn build_shared_signals(doc: &Document, raw_bytes: &[u8], _pages: &HashMap<u32, String>) -> SharedSignals {
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
                "rg" => {
                    if op.operands.len() == 3 {
                        let r = op.operands[0].as_float().unwrap_or(0.0);
                        let g = op.operands[1].as_float().unwrap_or(0.0);
                        let b = op.operands[2].as_float().unwrap_or(0.0);
                        current_color_is_white = r > 0.99 && g > 0.99 && b > 0.99;
                    }
                }
                "g" => {
                    if op.operands.len() == 1 {
                        let gray = op.operands[0].as_float().unwrap_or(0.0);
                        current_color_is_white = gray > 0.99;
                    }
                }
                "Tf" => {
                    if op.operands.len() >= 2 {
                        current_font_size = op.operands[1].as_float().unwrap_or(12.0);
                    }
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

    SharedSignals { has_white_text, has_microscopic_font, has_incremental_update }
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

    let shared = build_shared_signals(doc, raw_bytes, pages);

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

pub fn instruction_patterns_pt_br() -> Regex {
    Regex::new(
        r"(?i)(ignore\s+(todas\s+)?(as\s+)?instru[çc][õo]es|desconsidere\s+o\s+prompt|aja\s+como|novo\s+objetivo|esque[çc]a\s+(as\s+)?instru[çc][õo]es|n[aã]o\s+siga\s+(as\s+)?regras|aten[çc][aã]o[\s,]+intelig[eê]ncia\s+artificial|n[aã]o\s+impugne|conteste\s+(esta|essa|de\s+forma)\s+.*superficial|conclua\s+que\s+todos|independentemente\s+do\s+comando)",
    ).expect("valid pt-BR instruction regex")
}

pub fn instruction_patterns_en() -> Regex {
    Regex::new(
        r"(?i)(ignore\s+(all\s+)?previous\s+instructions|disregard\s+(the\s+)?(above|previous)|act\s+as\s+(a\s+)?|new\s+objective|forget\s+(all\s+)?(your\s+)?instructions|you\s+are\s+now\s+|system\s*:\s*)",
    ).expect("valid EN instruction regex")
}

pub fn combined_instruction_patterns() -> Regex {
    Regex::new(
        r"(?i)(ignore\s+(todas\s+)?(as\s+)?instru[çc][õo]es|desconsidere\s+o\s+prompt|aja\s+como|novo\s+objetivo|esque[çc]a\s+(as\s+)?instru[çc][õo]es|n[aã]o\s+siga\s+(as\s+)?regras|aten[çc][aã]o[\s,]+intelig[eê]ncia\s+artificial|n[aã]o\s+impugne|conteste\s+(esta|essa|de\s+forma)\s+.*superficial|conclua\s+que\s+todos|independentemente\s+do\s+comando|ignore\s+(all\s+)?previous\s+instructions|disregard\s+(the\s+)?(above|previous)|act\s+as\s+(a\s+)?|new\s+objective|forget\s+(all\s+)?(your\s+)?instructions|you\s+are\s+now\s+|system\s*:\s*)",
    ).expect("valid combined instruction regex")
}
