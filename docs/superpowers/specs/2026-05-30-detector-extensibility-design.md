# Detector Extensibility Design

**Date:** 2026-05-30  
**Status:** Approved  
**Topic:** Refactor detection pipeline to a pluggable `VectorDetector` trait so new attack vectors can be added by creating one new file and registering it in one line.

---

## Problem

Adding a new detection vector currently requires touching 5+ files:
- `pdf_parser.rs` — add boolean fields to `PdfContent`
- `heuristic_detector.rs` — add logic to the monolithic `detect()` function
- `models.rs` — add variant to `DetectionType`
- `report_generator.rs` — add match arm to `translate_description()`
- `i18n/pt-BR.json` + `i18n/en.json` — add 3 keys each

There is no pattern or contract enforcing what a new detector must implement. The parser and heuristic logic for the same vector are split across two files.

---

## Goal

- Each detection vector lives entirely in one dedicated file (signals + heuristics)
- A `VectorDetector` trait acts as the template — the compiler tells you what to implement
- Adding a new vector requires: 1 new file + 1 enum variant + 1 registration line + i18n keys + 1 report match arm

---

## Architecture

### New Directory Structure

```
src-tauri/src/
├── detectors/                  ← new module (replaces heuristic_detector.rs)
│   ├── mod.rs                  ← VectorDetector trait + SharedSignals + run_all()
│   ├── white_text.rs           ← WhiteText + CitationPoisoning signals & heuristics
│   ├── invisible_text.rs
│   ├── microscopic_font.rs
│   ├── zero_width.rs
│   ├── ocg_layer.rs
│   ├── javascript.rs
│   ├── incremental_update.rs
│   ├── metadata.rs
│   ├── annotations.rs
│   ├── forms.rs
│   ├── actual_text.rs
│   ├── unicode_tricks.rs
│   ├── token_flooding.rs
│   ├── text_outside_bounds.rs
│   └── instruction_patterns.rs  ← handles InstructionPattern + ForeignLanguageInstruction
├── models.rs                   ← unchanged
├── pdf_parser.rs               ← slimmed: open doc, extract page text, return raw bytes
├── llm_analyzer.rs             ← unchanged
├── report_generator.rs         ← unchanged (existing match arms stay)
└── config.rs                   ← unchanged
```

### The `VectorDetector` Trait

```rust
pub trait VectorDetector: Send + Sync {
    /// Extract signals from the raw PDF document and bytes.
    fn extract(
        &self,
        doc: &lopdf::Document,
        raw_bytes: &[u8],
        pages: &HashMap<u32, String>,
    ) -> Box<dyn Any + Send>;

    /// Given extracted signals, produce findings.
    fn detect(
        &self,
        signals: &dyn Any,
        pages: &HashMap<u32, String>,
        shared: &SharedSignals,
    ) -> Vec<Finding>;

    /// Human-readable name used in logs and tests.
    fn name(&self) -> &'static str;
}
```

Each detector defines a concrete `Signals` struct. `extract()` populates it and returns it boxed as `dyn Any`. `detect()` downcasts it back to the concrete type.

### SharedSignals

Cross-detector dependencies (e.g. `CitationPoisoning` needs `has_white_text`) are handled via a thin `SharedSignals` struct populated in a first pass before individual detectors run:

```rust
pub struct SharedSignals {
    pub has_white_text: bool,
    pub has_microscopic_font: bool,
    pub has_incremental_update: bool,
}
```

`build_shared_signals()` in `detectors/mod.rs` runs a quick pre-pass over the document to populate this.

### The Registry (`detectors/mod.rs`)

```rust
pub fn run_all(
    doc: &lopdf::Document,
    raw_bytes: &[u8],
    pages: &HashMap<u32, String>,
) -> Vec<Finding> {
    let shared = build_shared_signals(doc, raw_bytes, pages);

    let detectors: Vec<Box<dyn VectorDetector>> = vec![
        Box::new(WhiteTextDetector),
        Box::new(InvisibleTextDetector),
        Box::new(MicroscopicFontDetector),
        Box::new(ZeroWidthDetector),
        Box::new(OcgLayerDetector),
        Box::new(JavaScriptDetector),
        Box::new(IncrementalUpdateDetector),
        Box::new(MetadataDetector),
        Box::new(AnnotationsDetector),
        Box::new(FormsDetector),
        Box::new(ActualTextDetector),
        Box::new(UnicodeTricksDetector),
        Box::new(TokenFloodingDetector),
        Box::new(TextOutsideBoundsDetector),
        Box::new(InstructionPatternsDetector),
        // ← register new detectors here
    ];

    detectors.iter().flat_map(|d| {
        let signals = d.extract(doc, raw_bytes, pages);
        d.detect(signals.as_ref(), pages, &shared)
    }).collect()
}
```

`lib.rs` calls `detectors::run_all()` instead of the old `heuristic_detector::detect()`.

### Slimmed `pdf_parser.rs`

After migration, `pdf_parser.rs` is responsible only for:
1. Opening the `lopdf::Document`
2. Extracting page text via `lopdf::extract_text()`
3. Reading raw bytes from disk

It no longer contains signal extraction functions (`extract_javascript`, `extract_ocg_hidden_texts`, `analyze_content_streams`, etc.) — those move into their respective detector files.

---

## Migration Map

| New file | Migrates from |
|---|---|
| `detectors/white_text.rs` | pdf_parser content stream analysis + heuristic (white text + citation poisoning) |
| `detectors/invisible_text.rs` | content stream Tr=3 detection + heuristic |
| `detectors/microscopic_font.rs` | content stream font size analysis + heuristic |
| `detectors/zero_width.rs` | `heuristic_detector::detect_zero_width_characters()` |
| `detectors/ocg_layer.rs` | `pdf_parser::extract_ocg_hidden_texts()` + heuristic |
| `detectors/javascript.rs` | `pdf_parser::extract_javascript()` + heuristic |
| `detectors/incremental_update.rs` | `pdf_parser::detect_incremental_update()` + heuristic |
| `detectors/metadata.rs` | `pdf_parser::extract_metadata()` + heuristic |
| `detectors/annotations.rs` | `pdf_parser::extract_annotations()` + heuristic |
| `detectors/forms.rs` | `pdf_parser::extract_acroform_fields()` + heuristic |
| `detectors/actual_text.rs` | `pdf_parser::extract_actual_text()` + heuristic |
| `detectors/text_outside_bounds.rs` | content stream coordinate analysis + heuristic |
| `detectors/unicode_tricks.rs` | `heuristic_detector::detect_unicode_tricks()` |
| `detectors/token_flooding.rs` | `heuristic_detector::detect_token_flooding()` |
| `detectors/instruction_patterns.rs` | `heuristic_detector::detect_instruction_patterns()` — handles both `InstructionPattern` (pt-BR) and `ForeignLanguageInstruction` (en) using the same regex engine |

After migration, `heuristic_detector.rs` is deleted.

---

## Adding a New Vector — Step-by-Step

### Step 1 — `models.rs`
Add one variant to `DetectionType`:
```rust
DetectionType::MyNewVector,
```

### Step 2 — `detectors/my_new_vector.rs` (new file)
All logic lives here — signals, extraction, heuristics:
```rust
use std::any::Any;
use std::collections::HashMap;
use crate::models::Finding;
use super::{VectorDetector, SharedSignals};

pub struct Signals {
    pub found_values: Vec<String>,
}

pub struct MyNewVectorDetector;

impl VectorDetector for MyNewVectorDetector {
    fn name(&self) -> &'static str { "my_new_vector" }

    fn extract(&self, doc: &lopdf::Document, raw_bytes: &[u8], pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // ... extract signals from doc/bytes/pages
        Box::new(Signals { found_values: vec![] })
    }

    fn detect(&self, signals: &dyn Any, pages: &HashMap<u32, String>, shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<Signals>().unwrap();
        // ... produce findings from signals
        vec![]
    }
}
```

### Step 3 — `detectors/mod.rs`
Register in the `run_all()` vec:
```rust
Box::new(MyNewVectorDetector),
```
Also add `mod my_new_vector;` and `use my_new_vector::MyNewVectorDetector;`.

### Step 4 — `i18n/pt-BR.json` + `i18n/en.json`
Add 3 keys per language:
```json
"detection_type_MyNewVector": "Display label",
"desc_MyNewVector": "Description shown in findings",
"info_MyNewVector": "Tooltip/info panel text"
```

### Step 5 — `report_generator.rs`
Add one match arm to `translate_description()`:
```rust
DetectionType::MyNewVector => "Report label",
```

---

## What Does Not Change

- `models.rs` types: `Finding`, `Severity`, `Verdict`, `AnalysisResult`, `LlmClassification`, `AppConfig`
- `llm_analyzer.rs`
- `report_generator.rs` existing match arms
- `config.rs`
- All frontend code (`src/`)
- All existing i18n keys

---

## Testing

- Each detector file contains its own `#[cfg(test)]` module
- Integration tests in `heuristic_detector.rs` are migrated to their respective detector files
- Existing sample PDFs in `samples/vectors/` continue to serve as integration test fixtures
- The `make test` / `cargo test --all` command is unchanged

---

## Files Deleted

- `src-tauri/src/heuristic_detector.rs` — fully replaced by `detectors/`
