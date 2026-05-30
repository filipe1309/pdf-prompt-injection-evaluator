use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfParseError {
    #[error("Failed to open PDF: {0}")]
    OpenError(String),
    #[error("Failed to extract text: {0}")]
    ExtractionError(String),
}

pub struct ParsedPdf {
    pub doc: lopdf::Document,
    pub raw_bytes: Vec<u8>,
    pub pages: HashMap<u32, String>,
}

pub fn load_pdf(path: &Path) -> Result<ParsedPdf, PdfParseError> {
    let raw_bytes = std::fs::read(path)
        .map_err(|e| PdfParseError::OpenError(e.to_string()))?;
    let doc = lopdf::Document::load(path)
        .map_err(|e| PdfParseError::OpenError(e.to_string()))?;

    let mut pages = HashMap::new();
    for page_num in doc.get_pages().keys().copied() {
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        pages.insert(page_num, text);
    }

    Ok(ParsedPdf { doc, raw_bytes, pages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn test_pdf_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-artifacts");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn create_minimal_pdf(path: &Path) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![100.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal("Hello World")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }

    #[test]
    fn test_load_valid_pdf_extracts_text() {
        let test_path = test_pdf_path("test_clean_load.pdf");
        create_minimal_pdf(&test_path);

        let result = load_pdf(&test_path);

        fs::remove_file(&test_path).unwrap();
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(!parsed.pages.is_empty());
        assert!(parsed.pages.values().any(|text| text.contains("Hello World")));
    }

    #[test]
    fn test_load_nonexistent_pdf_returns_error() {
        let result = load_pdf(Path::new("definitely-missing.pdf"));
        assert!(result.is_err());
    }
}
