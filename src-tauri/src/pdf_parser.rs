use lopdf::{Document, Object};
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

pub struct PdfContent {
    pub pages: HashMap<u32, String>,
    pub metadata: HashMap<String, String>,
    pub annotations: Vec<AnnotationInfo>,
    pub has_javascript: bool,
    pub has_white_text: bool,
}

pub struct AnnotationInfo {
    pub page: u32,
    pub content: String,
    pub annotation_type: String,
}

pub fn parse_pdf(path: &Path) -> Result<PdfContent, PdfParseError> {
    let doc = Document::load(path).map_err(|err| PdfParseError::OpenError(err.to_string()))?;

    let mut pages = HashMap::new();
    for page_num in doc.get_pages().keys().copied() {
        let text = doc.extract_text(&[page_num]).map_err(|err| {
            PdfParseError::ExtractionError(format!("page {page_num}: {err}"))
        })?;
        pages.insert(page_num, text);
    }

    Ok(PdfContent {
        pages,
        metadata: extract_metadata(&doc),
        annotations: extract_annotations(&doc),
        has_javascript: check_javascript(&doc),
        has_white_text: check_white_text(&doc),
    })
}

fn extract_metadata(doc: &Document) -> HashMap<String, String> {
    let mut metadata = HashMap::new();
    let info = match doc.trailer.get(b"Info") {
        Ok(info) => info,
        Err(_) => return metadata,
    };
    let info_dict = match doc.dereference(info) {
        Ok((_, object)) => match object.as_dict() {
            Ok(dict) => dict,
            Err(_) => return metadata,
        },
        Err(_) => return metadata,
    };

    for key in ["Title", "Author", "Subject", "Keywords", "Creator", "Producer"] {
        if let Ok(value) = info_dict.get(key.as_bytes()) {
            if let Ok(text) = value.as_string() {
                metadata.insert(key.to_string(), text.into_owned());
            }
        }
    }

    metadata
}

fn extract_annotations(doc: &Document) -> Vec<AnnotationInfo> {
    let mut annotations = Vec::new();

    for (page_num, page_id) in doc.get_pages() {
        let Ok(page_dict) = doc.get_dictionary(page_id) else {
            continue;
        };
        let Ok(annots_object) = page_dict.get(b"Annots") else {
            continue;
        };
        let annot_array = match doc.dereference(annots_object) {
            Ok((_, object)) => match object.as_array() {
                Ok(array) => array,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        for annot in annot_array {
            let annot_dict = match doc.dereference(annot) {
                Ok((_, object)) => match object.as_dict() {
                    Ok(dict) => dict,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            let content = annot_dict
                .get(b"Contents")
                .ok()
                .and_then(|value| value.as_string().ok())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            let annotation_type = annot_dict
                .get(b"Subtype")
                .ok()
                .and_then(|value| value.as_name_str().ok())
                .map(str::to_string)
                .unwrap_or_else(|| "Unknown".to_string());

            annotations.push(AnnotationInfo {
                page: page_num,
                content,
                annotation_type,
            });
        }
    }

    annotations
}

fn check_javascript(doc: &Document) -> bool {
    doc.objects.values().any(object_contains_javascript)
}

fn check_white_text(doc: &Document) -> bool {
    use lopdf::content::Content;

    for (page_num, page_id) in doc.get_pages() {
        let Ok(content_data) = doc.get_page_content(page_id) else {
            continue;
        };
        let Ok(content) = Content::decode(&content_data) else {
            continue;
        };

        let mut current_color_is_white = false;
        let mut has_text_while_white = false;

        for op in &content.operations {
            match op.operator.as_str() {
                // Non-stroking color (fill) - RGB
                "rg" => {
                    if op.operands.len() == 3 {
                        let r = op.operands[0].as_float().unwrap_or(0.0);
                        let g = op.operands[1].as_float().unwrap_or(0.0);
                        let b = op.operands[2].as_float().unwrap_or(0.0);
                        current_color_is_white = r > 0.99 && g > 0.99 && b > 0.99;
                    }
                }
                // Gray colorspace
                "g" => {
                    if op.operands.len() == 1 {
                        let gray = op.operands[0].as_float().unwrap_or(0.0);
                        current_color_is_white = gray > 0.99;
                    }
                }
                // Text operators
                "Tj" | "TJ" | "'" | "\"" => {
                    if current_color_is_white {
                        has_text_while_white = true;
                    }
                }
                _ => {}
            }
        }

        if has_text_while_white {
            let _ = page_num; // suppress unused warning
            return true;
        }
    }
    false
}

fn object_contains_javascript(object: &Object) -> bool {
    match object {
        Object::Dictionary(dict) => {
            dict.has(b"JS")
                || dict.has(b"JavaScript")
                || dict.iter().any(|(_, value)| object_contains_javascript(value))
        }
        Object::Stream(stream) => {
            stream.dict.has(b"JS")
                || stream.dict.has(b"JavaScript")
                || stream
                    .dict
                    .iter()
                    .any(|(_, value)| object_contains_javascript(value))
        }
        Object::Array(items) => items.iter().any(object_contains_javascript),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Object, Stream};
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
    fn test_parse_valid_pdf_extracts_text() {
        let test_path = test_pdf_path("test_clean.pdf");
        create_minimal_pdf(&test_path);

        let result = parse_pdf(&test_path);

        fs::remove_file(&test_path).unwrap();
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(!content.pages.is_empty());
        assert!(content.pages.values().any(|text| text.contains("Hello World")));
        assert!(!content.has_javascript);
    }

    #[test]
    fn test_parse_nonexistent_pdf_returns_error() {
        let result = parse_pdf(Path::new("definitely-missing.pdf"));
        assert!(result.is_err());
    }
}
