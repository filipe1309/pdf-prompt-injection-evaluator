use crate::models::{AnalysisResult, DetectionType, Finding, LlmClassification, Severity, Verdict};
use genpdf::Element as _;
use genpdf::{elements, fonts, style, Alignment};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

struct ReportLabels {
    title: &'static str,
    disclaimer: &'static str,
    file_info: &'static str,
    filename: &'static str,
    analysis_date: &'static str,
    hash: &'static str,
    verdict: &'static str,
    verdict_safe: &'static str,
    verdict_unsafe: &'static str,
    findings: &'static str,
    no_findings: &'static str,
    severity_label: &'static str,
    page_label: &'static str,
    description_label: &'static str,
    excerpt_label: &'static str,
    offset_label: &'static str,
    severity_critical: &'static str,
    severity_warning: &'static str,
    severity_clean: &'static str,
    llm_analysis: &'static str,
    classification: &'static str,
    confidence: &'static str,
    explanation: &'static str,
    footer: &'static str,
    version: &'static str,
}

fn labels_for(language: &str) -> ReportLabels {
    if language.starts_with("pt") {
        ReportLabels {
            title: "RELAT\u{d3}RIO DE AN\u{c1}LISE DE INJE\u{c7}\u{c3}O EM PDF",
            disclaimer: "Este relat\u{f3}rio \u{e9} apenas consultivo e deve ser avaliado junto ao documento original e ao julgamento do analista.",
            file_info: "INFORMA\u{c7}\u{d5}ES DO ARQUIVO",
            filename: "Arquivo",
            analysis_date: "Data da an\u{e1}lise",
            hash: "Hash SHA-256",
            verdict: "VEREDITO",
            verdict_safe: "SEGURO",
            verdict_unsafe: "INSEGURO",
            findings: "ACHADOS",
            no_findings: "Nenhuma ocorr\u{ea}ncia detectada.",
            severity_label: "Severidade",
            page_label: "P\u{e1}gina",
            description_label: "Descri\u{e7}\u{e3}o",
            excerpt_label: "Trecho",
            offset_label: "Offset",
            severity_critical: "CR\u{cd}TICO",
            severity_warning: "ALERTA",
            severity_clean: "LIMPO",
            llm_analysis: "AN\u{c1}LISE LLM",
            classification: "Classifica\u{e7}\u{e3}o",
            confidence: "Confian\u{e7}a",
            explanation: "Explica\u{e7}\u{e3}o",
            footer: "RODAP\u{c9}",
            version: "Vers\u{e3}o",
        }
    } else {
        ReportLabels {
            title: "PDF INJECTION ANALYSIS REPORT",
            disclaimer: "This report is advisory only and should be reviewed alongside the source document and analyst judgment.",
            file_info: "FILE INFORMATION",
            filename: "Filename",
            analysis_date: "Analysis date",
            hash: "SHA-256 hash",
            verdict: "VERDICT",
            verdict_safe: "SAFE",
            verdict_unsafe: "UNSAFE",
            findings: "FINDINGS",
            no_findings: "No findings detected.",
            severity_label: "Severity",
            page_label: "Page",
            description_label: "Description",
            excerpt_label: "Excerpt",
            offset_label: "Character offset",
            severity_critical: "CRITICAL",
            severity_warning: "WARNING",
            severity_clean: "CLEAN",
            llm_analysis: "LLM ANALYSIS",
            classification: "Classification",
            confidence: "Confidence",
            explanation: "Explanation",
            footer: "FOOTER",
            version: "Version",
        }
    }
}

fn translate_description(detection_type: &DetectionType, original: &str, language: &str) -> String {
    if !language.starts_with("pt") {
        return original.to_string();
    }
    match detection_type {
        DetectionType::ZeroWidthChars => "Caracteres de largura zero detectados no texto".to_string(),
        DetectionType::InvisibleText => "Texto invis\u{ed}vel detectado (modo de renderiza\u{e7}\u{e3}o oculto)".to_string(),
        DetectionType::WhiteText => "Texto em branco (invis\u{ed}vel ao leitor) com instru\u{e7}\u{e3}o oculta detectada".to_string(),
        DetectionType::MicroscopicFont => "Fonte microscopica (<2pt) detectada -- texto ilegivel".to_string(),
        DetectionType::TextOutsideBounds => "Texto posicionado fora dos limites vis\u{ed}veis da p\u{e1}gina".to_string(),
        DetectionType::HiddenAnnotation => "Anota\u{e7}\u{e3}o oculta com conte\u{fa}do suspeito detectada".to_string(),
        DetectionType::HiddenFormField => "Campo de formul\u{e1}rio oculto com instru\u{e7}\u{f5}es detectado".to_string(),
        DetectionType::InstructionPattern => "Padr\u{e3}o de instru\u{e7}\u{e3}o de prompt injection detectado no texto".to_string(),
        DetectionType::UnicodeTrick => "Truque Unicode (BiDi override) detectado".to_string(),
        DetectionType::EmbeddedJavaScript => "JavaScript embutido detectado no documento PDF".to_string(),
        DetectionType::MetadataInjection => "Inje\u{e7}\u{e3}o detectada em metadados do PDF".to_string(),
        DetectionType::TokenFlooding => "Inunda\u{e7}\u{e3}o de tokens detectada (texto oculto repetitivo)".to_string(),
        DetectionType::ForeignLanguageInstruction => "Instru\u{e7}\u{e3}o em idioma estrangeiro detectada".to_string(),
        DetectionType::IncrementalUpdate => "Revisao incremental detectada -- conteudo adicionado apos estrutura original".to_string(),
        DetectionType::ActualTextInjection => "Inje\u{e7}\u{e3}o via ActualText detectada".to_string(),
        DetectionType::HiddenOcgLayer => "Camada OCG oculta com texto suspeito detectada".to_string(),
        DetectionType::CitationPoisoning => "Cita\u{e7}\u{e3}o jur\u{ed}dica fabricada detectada".to_string(),
    }
}

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
    language: &str,
) -> Result<(), ReportError> {
    ensure_parent_dir(output_path)?;

    match try_generate_pdf(result, llm_result, output_path, language) {
        Ok(()) => Ok(()),
        Err(pdf_error) => generate_text_report(result, llm_result, output_path, language).map_err(|text_error| {
            ReportError::GenerationError(format!(
                "PDF generation failed ({pdf_error}); text fallback failed ({text_error})"
            ))
        }),
    }
}

pub fn generate_batch_report(
    results: &[AnalysisResult],
    output_path: &Path,
    language: &str,
) -> Result<(), ReportError> {
    ensure_parent_dir(output_path)?;

    match try_generate_batch_pdf(results, output_path, language) {
        Ok(()) => Ok(()),
        Err(pdf_error) => generate_batch_text_report(results, output_path, language).map_err(|text_error| {
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
    language: &str,
) -> Result<(), ReportError> {
    let labels = labels_for(language);
    let font_family = load_font_family()?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(labels.title);
    doc.set_minimal_conformance();
    doc.set_line_spacing(1.4);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);

    // Title
    doc.push(
        elements::Paragraph::new(labels.title)
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(0.5));

    // Separator line
    doc.push(elements::Paragraph::new(
        "_".repeat(80),
    ).styled(style::Style::new().with_font_size(6).with_color(style::Color::Rgb(180, 180, 180))));
    doc.push(elements::Break::new(1.0));

    // File info section using table layout for alignment
    push_section_title(&mut doc, labels.file_info);
    doc.push(elements::Break::new(0.3));

    let mut table = elements::TableLayout::new(vec![1, 3]);
    table.push_row(vec![
        Box::new(elements::Paragraph::new(labels.filename)
            .styled(style::Style::new().bold().with_font_size(9))),
        Box::new(elements::Paragraph::new(&result.filename)
            .styled(style::Style::new().with_font_size(9))),
    ]).expect("table row");
    table.push_row(vec![
        Box::new(elements::Paragraph::new(labels.analysis_date)
            .styled(style::Style::new().bold().with_font_size(9))),
        Box::new(elements::Paragraph::new(&result.analyzed_at)
            .styled(style::Style::new().with_font_size(9))),
    ]).expect("table row");
    table.push_row(vec![
        Box::new(elements::Paragraph::new(labels.hash)
            .styled(style::Style::new().bold().with_font_size(9))),
        Box::new(elements::Paragraph::new(&result.file_hash)
            .styled(style::Style::new().with_font_size(8))),
    ]).expect("table row");
    doc.push(table);
    doc.push(elements::Break::new(1.2));

    // Verdict section with colored text
    push_section_title(&mut doc, labels.verdict);
    doc.push(elements::Break::new(0.3));
    let (verdict_text, verdict_color) = match result.verdict {
        Verdict::Safe => (labels.verdict_safe, style::Color::Rgb(34, 139, 34)),
        Verdict::Unsafe => (labels.verdict_unsafe, style::Color::Rgb(200, 40, 40)),
    };
    doc.push(
        elements::Paragraph::new(verdict_text)
            .styled(style::Style::new().bold().with_font_size(14).with_color(verdict_color)),
    );
    doc.push(elements::Break::new(1.2));

    // Findings section
    push_section_title(&mut doc, labels.findings);
    doc.push(elements::Break::new(0.3));
    if result.findings.is_empty() {
        doc.push(
            elements::Paragraph::new(labels.no_findings)
                .styled(style::Style::new().italic().with_font_size(10)
                    .with_color(style::Color::Rgb(100, 100, 100))),
        );
    } else {
        for (index, finding) in result.findings.iter().enumerate() {
            push_finding_block(&mut doc, index + 1, finding, &labels, language);
        }
    }
    doc.push(elements::Break::new(1.0));

    // LLM analysis section
    if let Some(llm_result) = llm_result {
        push_section_title(&mut doc, labels.llm_analysis);
        doc.push(elements::Break::new(0.3));

        let mut table = elements::TableLayout::new(vec![1, 3]);
        table.push_row(vec![
            Box::new(elements::Paragraph::new(labels.classification)
                .styled(style::Style::new().bold().with_font_size(9))),
            Box::new(elements::Paragraph::new(&llm_result.classification)
                .styled(style::Style::new().with_font_size(9))),
        ]).expect("table row");
        table.push_row(vec![
            Box::new(elements::Paragraph::new(labels.confidence)
                .styled(style::Style::new().bold().with_font_size(9))),
            Box::new(elements::Paragraph::new(format!("{}%", llm_result.confidence))
                .styled(style::Style::new().with_font_size(9))),
        ]).expect("table row");
        table.push_row(vec![
            Box::new(elements::Paragraph::new(labels.explanation)
                .styled(style::Style::new().bold().with_font_size(9))),
            Box::new(elements::Paragraph::new(&llm_result.explanation)
                .styled(style::Style::new().with_font_size(9))),
        ]).expect("table row");
        doc.push(table);
        doc.push(elements::Break::new(1.0));
    }

    // Footer separator
    doc.push(elements::Paragraph::new(
        "_".repeat(80),
    ).styled(style::Style::new().with_font_size(6).with_color(style::Color::Rgb(180, 180, 180))));
    doc.push(elements::Break::new(0.3));

    // Footer
    doc.push(
        elements::Paragraph::new(format!("{}: {}", labels.version, env!("CARGO_PKG_VERSION")))
            .styled(style::Style::new().with_font_size(8).with_color(style::Color::Rgb(120, 120, 120))),
    );
    doc.push(
        elements::Paragraph::new(labels.disclaimer)
            .styled(style::Style::new().italic().with_font_size(8)
                .with_color(style::Color::Rgb(120, 120, 120))),
    );

    doc.render_to_file(output_path)
        .map_err(|e| ReportError::SaveError(e.to_string()))
}

fn push_finding_block(doc: &mut genpdf::Document, index: usize, finding: &Finding, labels: &ReportLabels, language: &str) {
    let (severity_text, severity_color) = match finding.severity {
        Severity::Critical => (labels.severity_critical, style::Color::Rgb(200, 40, 40)),
        Severity::Warning => (labels.severity_warning, style::Color::Rgb(200, 140, 0)),
        Severity::Clean => (labels.severity_clean, style::Color::Rgb(34, 139, 34)),
    };

    // Finding header: "1. CRÍTICO — Página 1"
    let mut header = elements::Paragraph::new(format!("{}. ", index));
    header.push_styled(
        format!("{}", severity_text),
        style::Style::new().bold().with_color(severity_color),
    );
    header.push_styled(
        format!(" -- {} {}", labels.page_label, finding.page),
        style::Style::new().with_color(style::Color::Rgb(80, 80, 80)),
    );
    doc.push(header.styled(style::Style::new().with_font_size(10)));

    // Description (translated)
    let description = translate_description(&finding.detection_type, &finding.description, language);
    doc.push(
        elements::PaddedElement::new(
            elements::Paragraph::new(description)
                .styled(style::Style::new().with_font_size(9)),
            genpdf::Margins::trbl(1, 0, 1, 12),
        ),
    );

    // Excerpt (if present)
    let excerpt = finding.excerpt.trim();
    if !excerpt.is_empty() {
        let excerpt_display = if excerpt.len() > 120 {
            format!("{}...", &excerpt[..120])
        } else {
            excerpt.to_string()
        };
        doc.push(
            elements::PaddedElement::new(
                elements::Paragraph::new(format!("\"{}\"", excerpt_display))
                    .styled(style::Style::new().italic().with_font_size(8)
                        .with_color(style::Color::Rgb(80, 80, 80))),
                genpdf::Margins::trbl(0, 0, 2, 12),
            ),
        );
    }

    doc.push(elements::Break::new(0.4));
}

fn try_generate_batch_pdf(
    results: &[AnalysisResult],
    output_path: &Path,
    language: &str,
) -> Result<(), ReportError> {
    let labels = labels_for(language);
    let font_family = load_font_family()?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(labels.title);
    doc.set_minimal_conformance();
    doc.set_line_spacing(1.4);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);

    // Title
    doc.push(
        elements::Paragraph::new(labels.title)
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(0.5));
    doc.push(elements::Paragraph::new(
        "_".repeat(80),
    ).styled(style::Style::new().with_font_size(6).with_color(style::Color::Rgb(180, 180, 180))));
    doc.push(elements::Break::new(1.0));

    // Summary table
    let safe_count = results.iter().filter(|r| r.verdict == Verdict::Safe).count();
    let unsafe_count = results.len() - safe_count;
    doc.push(elements::Paragraph::new(format!(
        "{}: {} | {}: {} | {}: {}",
        "Total", results.len(),
        labels.verdict_safe, safe_count,
        labels.verdict_unsafe, unsafe_count,
    )).styled(style::Style::new().bold().with_font_size(10)));
    doc.push(elements::Break::new(1.0));

    // Each file result
    for (i, result) in results.iter().enumerate() {
        push_section_title(&mut doc, &format!("{}. {}", i + 1, result.filename));
        doc.push(elements::Break::new(0.2));

        let (verdict_text, verdict_color) = match result.verdict {
            Verdict::Safe => (labels.verdict_safe, style::Color::Rgb(34, 139, 34)),
            Verdict::Unsafe => (labels.verdict_unsafe, style::Color::Rgb(200, 40, 40)),
        };
        doc.push(
            elements::Paragraph::new(verdict_text)
                .styled(style::Style::new().bold().with_font_size(11).with_color(verdict_color)),
        );

        if result.findings.is_empty() {
            doc.push(
                elements::Paragraph::new(labels.no_findings)
                    .styled(style::Style::new().italic().with_font_size(9)
                        .with_color(style::Color::Rgb(100, 100, 100))),
            );
        } else {
            for (idx, finding) in result.findings.iter().enumerate() {
                push_finding_block(&mut doc, idx + 1, finding, &labels, language);
            }
        }

        doc.push(elements::Break::new(0.5));
        doc.push(elements::Paragraph::new(
            "_".repeat(60),
        ).styled(style::Style::new().with_font_size(6).with_color(style::Color::Rgb(200, 200, 200))));
        doc.push(elements::Break::new(0.5));
    }

    // Footer
    doc.push(
        elements::Paragraph::new(format!("{}: {}", labels.version, env!("CARGO_PKG_VERSION")))
            .styled(style::Style::new().with_font_size(8).with_color(style::Color::Rgb(120, 120, 120))),
    );
    doc.push(
        elements::Paragraph::new(labels.disclaimer)
            .styled(style::Style::new().italic().with_font_size(8)
                .with_color(style::Color::Rgb(120, 120, 120))),
    );

    doc.render_to_file(output_path)
        .map_err(|e| ReportError::SaveError(e.to_string()))
}

fn generate_batch_text_report(
    results: &[AnalysisResult],
    output_path: &Path,
    language: &str,
) -> Result<(), ReportError> {
    let labels = labels_for(language);
    let mut content = String::new();

    content.push_str(labels.title);
    content.push_str("\n================================\n\n");

    let safe_count = results.iter().filter(|r| r.verdict == Verdict::Safe).count();
    let unsafe_count = results.len() - safe_count;
    content.push_str(&format!(
        "Total: {} | {}: {} | {}: {}\n\n",
        results.len(), labels.verdict_safe, safe_count, labels.verdict_unsafe, unsafe_count
    ));

    for (i, result) in results.iter().enumerate() {
        content.push_str(&format!("{}. {}\n", i + 1, result.filename));
        content.push_str(&format!("   {}: {}\n", labels.hash, result.file_hash));
        let verdict_text = match result.verdict {
            Verdict::Safe => labels.verdict_safe,
            Verdict::Unsafe => labels.verdict_unsafe,
        };
        content.push_str(&format!("   {}: {}\n", labels.verdict, verdict_text));

        if result.findings.is_empty() {
            content.push_str(&format!("   {}\n", labels.no_findings));
        } else {
            for (idx, finding) in result.findings.iter().enumerate() {
                content.push_str(&format!("   {}\n", format_finding(idx + 1, finding, &labels, language)));
            }
        }
        content.push_str("\n---\n\n");
    }

    content.push_str(&format!("{}: {}\n", labels.version, env!("CARGO_PKG_VERSION")));
    content.push_str(&format!("{}\n", labels.disclaimer));

    fs::write(output_path, content)
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
    language: &str,
) -> Result<(), ReportError> {
    fs::write(output_path, build_text_report(result, llm_result, language))
        .map_err(|e| ReportError::SaveError(e.to_string()))
}

fn build_text_report(result: &AnalysisResult, llm_result: Option<&LlmClassification>, language: &str) -> String {
    let labels = labels_for(language);
    let mut content = String::new();

    content.push_str(labels.title);
    content.push_str("\n================================\n\n");

    content.push_str(labels.file_info);
    content.push_str("\n----------------\n");
    content.push_str(&format!("{}: {}\n", labels.filename, result.filename));
    content.push_str(&format!("{}: {}\n", labels.analysis_date, result.analyzed_at));
    content.push_str(&format!("{}: {}\n\n", labels.hash, result.file_hash));

    content.push_str(labels.verdict);
    content.push_str("\n-------\n");
    let verdict_text = match result.verdict {
        Verdict::Safe => labels.verdict_safe,
        Verdict::Unsafe => labels.verdict_unsafe,
    };
    content.push_str(&format!("{}\n\n", verdict_text));

    content.push_str(labels.findings);
    content.push_str("\n--------\n");
    if result.findings.is_empty() {
        content.push_str(&format!("{}\n\n", labels.no_findings));
    } else {
        for (index, finding) in result.findings.iter().enumerate() {
            content.push_str(&format!("{}\n\n", format_finding(index + 1, finding, &labels, language)));
        }
    }

    if let Some(llm_result) = llm_result {
        content.push_str(labels.llm_analysis);
        content.push_str("\n------------\n");
        content.push_str(&format!("{}: {}\n", labels.classification, llm_result.classification));
        content.push_str(&format!("{}: {}%\n", labels.confidence, llm_result.confidence));
        content.push_str(&format!("{}: {}\n\n", labels.explanation, llm_result.explanation));
    }

    content.push_str(labels.footer);
    content.push_str("\n------\n");
    content.push_str(&format!("{}: {}\n", labels.version, env!("CARGO_PKG_VERSION")));
    content.push_str(&format!("{}\n", labels.disclaimer));

    content
}

fn push_section_title(doc: &mut genpdf::Document, title: &str) {
    doc.push(
        elements::Paragraph::new(title)
            .styled(style::Style::new().bold().with_font_size(12)
                .with_color(style::Color::Rgb(40, 40, 80))),
    );
}

fn format_finding(index: usize, finding: &Finding, labels: &ReportLabels, language: &str) -> String {
    let offset = finding
        .char_offset
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let excerpt = if finding.excerpt.trim().is_empty() {
        "N/A"
    } else {
        finding.excerpt.trim()
    };
    let severity_text = match finding.severity {
        Severity::Critical => labels.severity_critical,
        Severity::Warning => labels.severity_warning,
        Severity::Clean => labels.severity_clean,
    };
    let description = translate_description(&finding.detection_type, &finding.description, language);

    format!(
        "{index}. {}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}",
        labels.severity_label, severity_text,
        labels.page_label, finding.page,
        labels.description_label, description,
        labels.excerpt_label, excerpt,
        labels.offset_label, offset,
    )
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
            extracted_text: "hidden text here".to_string(),
        };

        let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("report-tests");
        fs::create_dir_all(&output_dir).unwrap();
        let output = output_dir.join("test_injection_report.pdf");

        let res = generate_report(&result, None, &output, "pt-BR");
        assert!(res.is_ok());
        assert!(output.exists());
        fs::remove_file(&output).ok();
    }
}
