use crate::models::{AnalysisResult, Finding, LlmClassification, Severity, Verdict};
use genpdf::Element as _;
use genpdf::{elements, fonts, style, Alignment};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const REPORT_TITLE: &str = "PDF INJECTION ANALYSIS REPORT";
const DISCLAIMER: &str =
    "This report is advisory only and should be reviewed alongside the source document and analyst judgment.";

#[derive(Error, Debug)]
pub enum ReportError {
    #[error("Failed to generate report: {0}")]
    GenerationError(String),
    #[error("Failed to save report: {0}")]
    SaveError(String),
}

pub fn generate_report(
    result: &AnalysisResult,
    llm_result: Option<&LlmClassification>,
    output_path: &Path,
) -> Result<(), ReportError> {
    ensure_parent_dir(output_path)?;

    match try_generate_pdf(result, llm_result, output_path) {
        Ok(()) => Ok(()),
        Err(pdf_error) => generate_text_report(result, llm_result, output_path).map_err(|text_error| {
            ReportError::GenerationError(format!(
                "PDF generation failed ({pdf_error}); text fallback failed ({text_error})"
            ))
        }),
    }
}

fn ensure_parent_dir(output_path: &Path) -> Result<(), ReportError> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ReportError::SaveError(e.to_string()))?;
    }
    Ok(())
}

fn try_generate_pdf(
    result: &AnalysisResult,
    llm_result: Option<&LlmClassification>,
    output_path: &Path,
) -> Result<(), ReportError> {
    let font_family = load_font_family()?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(REPORT_TITLE);
    doc.set_minimal_conformance();

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new(REPORT_TITLE)
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Break::new(1.5));

    push_section_title(&mut doc, "FILE INFORMATION");
    doc.push(elements::Paragraph::new(format!("Filename: {}", result.filename)));
    doc.push(elements::Paragraph::new(format!(
        "Analysis date: {}",
        result.analyzed_at
    )));
    doc.push(elements::Paragraph::new(format!("SHA-256 hash: {}", result.file_hash)));
    doc.push(elements::Break::new(1.0));

    push_section_title(&mut doc, "VERDICT");
    doc.push(
        elements::Paragraph::new(format_verdict(&result.verdict))
            .styled(style::Style::new().bold().with_font_size(14)),
    );
    doc.push(elements::Break::new(1.0));

    push_section_title(&mut doc, "FINDINGS");
    if result.findings.is_empty() {
        doc.push(elements::Paragraph::new("No findings detected."));
    } else {
        for (index, finding) in result.findings.iter().enumerate() {
            doc.push(elements::Paragraph::new(format_finding(index + 1, finding)));
            doc.push(elements::Break::new(0.8));
        }
    }
    doc.push(elements::Break::new(1.0));

    if let Some(llm_result) = llm_result {
        push_section_title(&mut doc, "LLM ANALYSIS");
        doc.push(elements::Paragraph::new(format!(
            "Classification: {}",
            llm_result.classification
        )));
        doc.push(elements::Paragraph::new(format!(
            "Confidence: {}%",
            llm_result.confidence
        )));
        doc.push(elements::Paragraph::new(format!(
            "Explanation: {}",
            llm_result.explanation
        )));
        doc.push(elements::Break::new(1.0));
    }

    push_section_title(&mut doc, "FOOTER");
    doc.push(elements::Paragraph::new(format!(
        "Version: {}",
        env!("CARGO_PKG_VERSION")
    )));
    doc.push(elements::Paragraph::new(format!("Disclaimer: {DISCLAIMER}")));

    doc.render_to_file(output_path)
        .map_err(|e| ReportError::SaveError(e.to_string()))
}

fn load_font_family() -> Result<fonts::FontFamily<fonts::FontData>, ReportError> {
    let font_paths = [
        (PathBuf::from("/usr/share/fonts/truetype/liberation"), "LiberationSans"),
        (PathBuf::from("/usr/share/fonts/liberation-sans"), "LiberationSans"),
        (PathBuf::from("."), "LiberationSans"),
    ];

    for (path, name) in font_paths {
        if let Ok(font_family) = fonts::from_files(&path, name, Some(fonts::Builtin::Helvetica)) {
            return Ok(font_family);
        }
    }

    load_macos_arial_family().map_err(|_| {
        ReportError::GenerationError("No usable font family found for PDF output".to_string())
    })
}

fn load_macos_arial_family() -> Result<fonts::FontFamily<fonts::FontData>, ReportError> {
    let dir = Path::new("/System/Library/Fonts/Supplemental");

    Ok(fonts::FontFamily {
        regular: fonts::FontData::load(dir.join("Arial.ttf"), None)
            .map_err(|e| ReportError::GenerationError(e.to_string()))?,
        bold: fonts::FontData::load(dir.join("Arial Bold.ttf"), None)
            .map_err(|e| ReportError::GenerationError(e.to_string()))?,
        italic: fonts::FontData::load(dir.join("Arial Italic.ttf"), None)
            .map_err(|e| ReportError::GenerationError(e.to_string()))?,
        bold_italic: fonts::FontData::load(dir.join("Arial Bold Italic.ttf"), None)
            .map_err(|e| ReportError::GenerationError(e.to_string()))?,
    })
}

fn generate_text_report(
    result: &AnalysisResult,
    llm_result: Option<&LlmClassification>,
    output_path: &Path,
) -> Result<(), ReportError> {
    fs::write(output_path, build_text_report(result, llm_result))
        .map_err(|e| ReportError::SaveError(e.to_string()))
}

fn build_text_report(result: &AnalysisResult, llm_result: Option<&LlmClassification>) -> String {
    let mut content = String::new();

    content.push_str(REPORT_TITLE);
    content.push_str("\n================================\n\n");

    content.push_str("FILE INFORMATION\n----------------\n");
    content.push_str(&format!("Filename: {}\n", result.filename));
    content.push_str(&format!("Analysis date: {}\n", result.analyzed_at));
    content.push_str(&format!("SHA-256 hash: {}\n\n", result.file_hash));

    content.push_str("VERDICT\n-------\n");
    content.push_str(&format!("{}\n\n", format_verdict(&result.verdict)));

    content.push_str("FINDINGS\n--------\n");
    if result.findings.is_empty() {
        content.push_str("No findings detected.\n\n");
    } else {
        for (index, finding) in result.findings.iter().enumerate() {
            content.push_str(&format!("{}\n\n", format_finding(index + 1, finding)));
        }
    }

    if let Some(llm_result) = llm_result {
        content.push_str("LLM ANALYSIS\n------------\n");
        content.push_str(&format!("Classification: {}\n", llm_result.classification));
        content.push_str(&format!("Confidence: {}%\n", llm_result.confidence));
        content.push_str(&format!("Explanation: {}\n\n", llm_result.explanation));
    }

    content.push_str("FOOTER\n------\n");
    content.push_str(&format!("Version: {}\n", env!("CARGO_PKG_VERSION")));
    content.push_str(&format!("Disclaimer: {}\n", DISCLAIMER));

    content
}

fn push_section_title(doc: &mut genpdf::Document, title: &str) {
    doc.push(
        elements::Paragraph::new(title).styled(style::Style::new().bold().with_font_size(13)),
    );
}

fn format_finding(index: usize, finding: &Finding) -> String {
    let offset = finding
        .char_offset
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let excerpt = if finding.excerpt.trim().is_empty() {
        "N/A"
    } else {
        finding.excerpt.trim()
    };

    format!(
        "{index}. Severity: {}\nPage: {}\nDescription: {}\nExcerpt: {}\nCharacter offset: {}",
        format_severity(&finding.severity),
        finding.page,
        finding.description,
        excerpt,
        offset,
    )
}

fn format_severity(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "CRITICAL",
        Severity::Warning => "WARNING",
        Severity::Clean => "CLEAN",
    }
}

fn format_verdict(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Safe => "SAFE",
        Verdict::Unsafe => "UNSAFE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DetectionType, Finding, Severity, Verdict};
    use std::path::PathBuf;

    #[test]
    fn test_generate_report_creates_file() {
        let result = AnalysisResult {
            verdict: Verdict::Unsafe,
            findings: vec![Finding {
                page: 1,
                severity: Severity::Critical,
                detection_type: DetectionType::ZeroWidthChars,
                description: "Zero-width characters detected".to_string(),
                excerpt: "hidden text here".to_string(),
                char_offset: Some(42),
            }],
            file_hash: "abc123def456".to_string(),
            filename: "test.pdf".to_string(),
            analyzed_at: "2026-05-28 12:00:00".to_string(),
        };

        let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("report-tests");
        fs::create_dir_all(&output_dir).unwrap();
        let output = output_dir.join("test_injection_report.pdf");

        let res = generate_report(&result, None, &output);
        assert!(res.is_ok());
        assert!(output.exists());
        fs::remove_file(&output).ok();
    }
}
