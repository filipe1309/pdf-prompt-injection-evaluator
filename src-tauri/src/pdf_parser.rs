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
    pub javascript_code: Option<String>,
    pub has_white_text: bool,
    pub has_invisible_text: bool,
    pub has_microscopic_font: bool,
    pub has_text_outside_bounds: bool,
    pub has_acroform_fields: bool,
    pub form_field_values: Vec<String>,
    pub has_incremental_update: bool,
    pub actual_text_values: Vec<String>,
    pub ocg_hidden_texts: Vec<String>,
}

pub struct AnnotationInfo {
    pub page: u32,
    pub content: String,
    pub annotation_type: String,
}

pub fn parse_pdf(path: &Path) -> Result<PdfContent, PdfParseError> {
    let has_incremental_update = detect_incremental_update(path);

    let doc = Document::load(path).map_err(|err| PdfParseError::OpenError(err.to_string()))?;

    let mut pages = HashMap::new();
    for page_num in doc.get_pages().keys().copied() {
        let text = doc.extract_text(&[page_num]).map_err(|err| {
            PdfParseError::ExtractionError(format!("page {page_num}: {err}"))
        })?;
        pages.insert(page_num, text);
    }

    let (has_white_text, has_invisible_text, has_microscopic_font, has_text_outside_bounds) = analyze_content_streams(&doc);
    let (has_acroform_fields, form_field_values) = extract_acroform_fields(&doc);
    let actual_text_values = extract_actual_text(&doc);
    let ocg_hidden_texts = extract_ocg_hidden_texts(&doc);

    Ok(PdfContent {
        pages,
        metadata: extract_metadata(&doc),
        annotations: extract_annotations(&doc),
        javascript_code: extract_javascript(&doc),
        has_white_text,
        has_invisible_text,
        has_microscopic_font,
        has_text_outside_bounds,
        has_acroform_fields,
        form_field_values,
        has_incremental_update,
        actual_text_values,
        ocg_hidden_texts,
    })
}

/// Detects incremental updates by counting %%EOF markers in raw bytes.
/// A valid single-revision PDF has exactly one %%EOF. Multiple markers indicate
/// incremental updates were appended (potentially after a digital signature).
fn detect_incremental_update(path: &Path) -> bool {
    let Ok(data) = std::fs::read(path) else {
        return false;
    };
    let mut count = 0;
    let marker = b"%%EOF";
    let mut pos = 0;
    while pos + marker.len() <= data.len() {
        if &data[pos..pos + marker.len()] == marker {
            count += 1;
            pos += marker.len();
        } else {
            pos += 1;
        }
    }
    count > 1
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

fn extract_javascript(doc: &Document) -> Option<String> {
    for (_id, obj) in doc.objects.iter() {
        if let Some(js) = extract_js_from_object(obj, doc, 0) {
            return Some(js);
        }
    }
    None
}

fn extract_js_from_object(object: &Object, doc: &Document, depth: u8) -> Option<String> {
    if depth > 5 { return None; }
    match object {
        Object::Dictionary(dict) => {
            if let Ok(js_obj) = dict.get(b"JS") {
                return extract_js_string(js_obj, doc);
            }
            if let Ok(js_obj) = dict.get(b"JavaScript") {
                return extract_js_string(js_obj, doc);
            }
            for (_, value) in dict.iter() {
                if let Some(js) = extract_js_from_object(value, doc, depth + 1) {
                    return Some(js);
                }
            }
            None
        }
        Object::Stream(stream) => {
            if let Ok(js_obj) = stream.dict.get(b"JS") {
                return extract_js_string(js_obj, doc);
            }
            if let Ok(js_obj) = stream.dict.get(b"JavaScript") {
                return extract_js_string(js_obj, doc);
            }
            for (_, value) in stream.dict.iter() {
                if let Some(js) = extract_js_from_object(value, doc, depth + 1) {
                    return Some(js);
                }
            }
            None
        }
        Object::Array(items) => {
            for item in items {
                if let Some(js) = extract_js_from_object(item, doc, depth + 1) {
                    return Some(js);
                }
            }
            None
        }
        Object::Reference(id) => {
            if let Ok(obj) = doc.get_object(*id) {
                extract_js_from_object(obj, doc, depth + 1)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_js_string(obj: &Object, doc: &Document) -> Option<String> {
    match obj {
        Object::String(bytes, _) => {
            let s = String::from_utf8_lossy(bytes).to_string();
            if !s.is_empty() { Some(s) } else { None }
        }
        Object::Stream(stream) => {
            stream.decompressed_content().ok()
                .and_then(|bytes| {
                    let s = String::from_utf8_lossy(&bytes).to_string();
                    if !s.is_empty() { Some(s) } else { None }
                })
        }
        Object::Reference(id) => {
            if let Ok(resolved) = doc.get_object(*id) {
                extract_js_string(resolved, doc)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn analyze_content_streams(doc: &Document) -> (bool, bool, bool, bool) {
    use lopdf::content::Content;

    let mut has_white_text = false;
    let mut has_invisible_text = false;
    let mut has_microscopic_font = false;
    let mut has_text_outside_bounds = false;

    for (_page_num, page_id) in doc.get_pages() {
        // Get page MediaBox for bounds checking
        let media_box = doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|d| d.get(b"MediaBox").ok())
            .and_then(|obj| doc.dereference(obj).ok())
            .and_then(|(_, obj)| obj.as_array().ok().cloned())
            .unwrap_or_default();

        let (page_width, page_height) = if media_box.len() == 4 {
            (
                media_box[2].as_float().unwrap_or(595.0f32),
                media_box[3].as_float().unwrap_or(842.0f32),
            )
        } else {
            (595.0f32, 842.0f32) // A4 default
        };

        let Ok(content_data) = doc.get_page_content(page_id) else {
            continue;
        };
        let Ok(content) = Content::decode(&content_data) else {
            continue;
        };

        let mut current_color_is_white = false;
        let mut current_font_size: f32 = 12.0;
        let mut current_x: f32 = 0.0;
        let mut current_y: f32 = 0.0;
        let mut current_render_mode: i32 = 0;

        for op in &content.operations {
            match op.operator.as_str() {
                // Non-stroking color RGB
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
                // Text rendering mode (Tr): 3 = invisible
                "Tr" => {
                    if op.operands.len() >= 1 {
                        current_render_mode = op.operands[0].as_i64().unwrap_or(0) as i32;
                    }
                }
                // Font size (Tf operator: /FontName size Tf)
                "Tf" => {
                    if op.operands.len() >= 2 {
                        current_font_size = op.operands[1].as_float().unwrap_or(12.0);
                    }
                }
                // Text position (Td, TD)
                "Td" | "TD" => {
                    if op.operands.len() >= 2 {
                        current_x += op.operands[0].as_float().unwrap_or(0.0);
                        current_y += op.operands[1].as_float().unwrap_or(0.0);
                    }
                }
                // Text matrix (Tm)
                "Tm" => {
                    if op.operands.len() >= 6 {
                        current_x = op.operands[4].as_float().unwrap_or(0.0);
                        current_y = op.operands[5].as_float().unwrap_or(0.0);
                    }
                }
                // Begin text
                "BT" => {
                    current_x = 0.0;
                    current_y = 0.0;
                    current_render_mode = 0;
                }
                // Text operators
                "Tj" | "TJ" | "'" | "\"" => {
                    if current_color_is_white {
                        has_white_text = true;
                    }
                    if current_render_mode == 3 {
                        has_invisible_text = true;
                    }
                    if current_font_size <= 3.0 && current_font_size > 0.0 {
                        has_microscopic_font = true;
                    }
                    if current_x < -10.0 || current_x > page_width + 10.0
                        || current_y < -10.0 || current_y > page_height + 10.0
                    {
                        has_text_outside_bounds = true;
                    }
                }
                _ => {}
            }
        }
    }

    (has_white_text, has_invisible_text, has_microscopic_font, has_text_outside_bounds)
}

fn extract_acroform_fields(doc: &Document) -> (bool, Vec<String>) {
    let mut values = Vec::new();

    // Check for AcroForm in the catalog
    let catalog = match doc.trailer.get(b"Root") {
        Ok(root) => match doc.dereference(root) {
            Ok((_, obj)) => match obj.as_dict() {
                Ok(dict) => dict.clone(),
                Err(_) => return (false, values),
            },
            Err(_) => return (false, values),
        },
        Err(_) => return (false, values),
    };

    let acroform = match catalog.get(b"AcroForm") {
        Ok(obj) => match doc.dereference(obj) {
            Ok((_, obj)) => match obj.as_dict() {
                Ok(dict) => dict.clone(),
                Err(_) => return (false, values),
            },
            Err(_) => return (false, values),
        },
        Err(_) => return (false, values),
    };

    let fields = match acroform.get(b"Fields") {
        Ok(obj) => match doc.dereference(obj) {
            Ok((_, obj)) => match obj.as_array() {
                Ok(arr) => arr.clone(),
                Err(_) => return (false, values),
            },
            Err(_) => return (false, values),
        },
        Err(_) => return (false, values),
    };

    for field_ref in &fields {
        let field_dict = match doc.dereference(field_ref) {
            Ok((_, obj)) => match obj.as_dict() {
                Ok(dict) => dict,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Extract field value /V
        if let Ok(v) = field_dict.get(b"V") {
            if let Ok(text) = v.as_string() {
                values.push(text.into_owned());
            }
            // For signature fields, /V is a reference to a Sig dictionary
            // Extract /Reason, /Location, /ContactInfo which can hold injections
            if let Ok((_, deref_v)) = doc.dereference(v) {
                if let Ok(sig_dict) = deref_v.as_dict() {
                    for key in [b"Reason".as_slice(), b"Location", b"ContactInfo"] {
                        if let Ok(val) = sig_dict.get(key) {
                            if let Ok(text) = val.as_string() {
                                let s = text.into_owned();
                                if !s.trim().is_empty() {
                                    values.push(s);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let has_fields = !fields.is_empty();
    (has_fields, values)
}

/// Detects Optional Content Groups (OCG) that are set to OFF.
/// Hidden OCG layers are not rendered but their text IS extracted by parsers.
fn extract_ocg_hidden_texts(doc: &Document) -> Vec<String> {
    // Find OCG groups that are OFF in the default config
    let catalog = match doc.trailer.get(b"Root") {
        Ok(root) => match doc.dereference(root) {
            Ok((_, obj)) => match obj.as_dict() {
                Ok(dict) => dict.clone(),
                Err(_) => return vec![],
            },
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };

    let oc_props = match catalog.get(b"OCProperties") {
        Ok(obj) => match doc.dereference(obj) {
            Ok((_, obj)) => match obj.as_dict() {
                Ok(dict) => dict.clone(),
                Err(_) => return vec![],
            },
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };

    // Collect OFF OCG object IDs
    let mut off_ocg_ids: Vec<lopdf::ObjectId> = Vec::new();
    if let Ok(d) = oc_props.get(b"D") {
        if let Ok((_, d_obj)) = doc.dereference(d) {
            if let Ok(d_dict) = d_obj.as_dict() {
                if let Ok(off) = d_dict.get(b"OFF") {
                    if let Ok((_, off_obj)) = doc.dereference(off) {
                        if let Ok(arr) = off_obj.as_array() {
                            for item in arr {
                                if let Ok(reference) = item.as_reference() {
                                    off_ocg_ids.push(reference);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if off_ocg_ids.is_empty() {
        return vec![];
    }

    // Find XObjects (or streams) with /OC referencing an OFF OCG, extract their text
    let mut texts = Vec::new();
    for (_obj_id, obj) in doc.objects.iter() {
        if let Ok(dict) = obj.as_dict().or_else(|_| {
            if let Ok(stream) = obj.as_stream() {
                Ok(&stream.dict)
            } else {
                Err(lopdf::Error::Type)
            }
        }) {
            if let Ok(oc_ref) = dict.get(b"OC") {
                let oc_id = match oc_ref.as_reference() {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if off_ocg_ids.contains(&oc_id) {
                    // This object is hidden - try to extract text from its stream
                    if let Ok(stream) = obj.as_stream() {
                        if let Ok(content) = stream.decompressed_content() {
                            let text = extract_text_from_content_bytes(&content);
                            if !text.is_empty() {
                                texts.push(text);
                            }
                        }
                    }
                }
            }
        }
    }

    // Also look inside stream objects directly
    for (_obj_id, obj) in doc.objects.iter() {
        if let Ok(stream) = obj.as_stream() {
            if let Ok(oc_ref) = stream.dict.get(b"OC") {
                let oc_id = match oc_ref.as_reference() {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if off_ocg_ids.contains(&oc_id) {
                    if let Ok(content) = stream.decompressed_content() {
                        let text = extract_text_from_content_bytes(&content);
                        if !text.is_empty() && !texts.contains(&text) {
                            texts.push(text);
                        }
                    }
                }
            }
        }
    }

    texts
}

/// Extract text strings from raw PDF content stream bytes (parenthesized strings after Tj/TJ)
fn extract_text_from_content_bytes(content: &[u8]) -> String {
    let content_str = String::from_utf8_lossy(content);
    let mut result = String::new();

    // Match parenthesized strings: (text) Tj or [(text)] TJ
    let re = regex::Regex::new(r"\(([^)]*)\)\s*Tj").unwrap();
    for cap in re.captures_iter(&content_str) {
        if !result.is_empty() {
            result.push(' ');
        }
        // Unescape basic PDF string escapes
        let s = cap[1].replace("\\(", "(").replace("\\)", ")").replace("\\\\", "\\");
        result.push_str(&s);
    }

    result
}

/// Extracts /ActualText values from StructTreeRoot elements.
/// These accessibility attributes can contain text that differs from visible content,
/// enabling injection attacks where extractors read /ActualText instead of rendered text.
fn extract_actual_text(doc: &Document) -> Vec<String> {
    let mut values = Vec::new();

    for (_, object) in &doc.objects {
        collect_actual_text(object, doc, &mut values);
    }

    values
}

fn collect_actual_text(object: &Object, doc: &Document, values: &mut Vec<String>) {
    match object {
        Object::Dictionary(dict) => {
            if let Ok(actual_text) = dict.get(b"ActualText") {
                if let Ok(text) = actual_text.as_string() {
                    let s = text.into_owned();
                    if !s.trim().is_empty() {
                        values.push(s);
                    }
                }
            }
            // Also check /Alt (alternative text) which can be similarly abused
            if let Ok(alt) = dict.get(b"Alt") {
                if let Ok(text) = alt.as_string() {
                    let s = text.into_owned();
                    if !s.trim().is_empty() {
                        values.push(s);
                    }
                }
            }
        }
        Object::Stream(stream) => {
            if let Ok(actual_text) = stream.dict.get(b"ActualText") {
                if let Ok(text) = actual_text.as_string() {
                    let s = text.into_owned();
                    if !s.trim().is_empty() {
                        values.push(s);
                    }
                }
            }
        }
        Object::Reference(id) => {
            if let Ok(obj) = doc.get_object(*id) {
                collect_actual_text(obj, doc, values);
            }
        }
        _ => {}
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
        assert!(content.javascript_code.is_none());
    }

    #[test]
    fn test_parse_nonexistent_pdf_returns_error() {
        let result = parse_pdf(Path::new("definitely-missing.pdf"));
        assert!(result.is_err());
    }
}
