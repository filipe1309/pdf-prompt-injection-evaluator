# AGENTS.md

## Project Overview

PDF Prompt Injection Evaluator — a Tauri v2 desktop app that detects hidden prompt injection attacks in PDF documents. Built for Brazilian lawyers to verify judicial PDFs before AI analysis.

## Architecture

- **Backend**: Rust (Tauri v2) in `src-tauri/src/`
- **Frontend**: Vanilla HTML/CSS/JS in `src/`
- **PDF Parsing**: `lopdf` crate for low-level PDF object manipulation
- **i18n**: JSON translation files in `src/i18n/` (pt-BR primary, en secondary)

### Key Modules

| File | Responsibility |
|------|---------------|
| `src-tauri/src/models.rs` | Data types: `DetectionType` enum, `Finding`, `Severity`, `AnalysisResult` (includes `extracted_text`) |
| `src-tauri/src/pdf_parser.rs` | PDF parsing: opens document, extracts page text. Exposes `load_pdf() -> ParsedPdf` |
| `src-tauri/src/detectors/mod.rs` | `VectorDetector` trait, `SharedSignals`, `run_all()` registry, shared regex helpers |
| `src-tauri/src/detectors/<vector>.rs` | One file per attack vector — owns signal extraction + heuristic logic (15 files) |
| `src-tauri/src/llm_analyzer.rs` | Optional LLM-based semantic analysis (Layer 2) — receives full text + findings context |
| `src-tauri/src/report_generator.rs` | PDF/text report generation (single + batch), translated descriptions per language |
| `src-tauri/src/config.rs` | Persistent app configuration (OS config dir) |
| `src/app.js` | Frontend: UI rendering, i18n, file queue, batch deep analysis, Lucide icons |
| `src/i18n/pt-BR.json` | Portuguese translations (primary language) |
| `src/i18n/en.json` | English translations |

### Detector Modules (`src-tauri/src/detectors/`)

Each attack vector lives in its own file implementing the `VectorDetector` trait:

| File | DetectionType(s) |
|------|-----------------|
| `white_text.rs` | `WhiteText`, `CitationPoisoning` |
| `invisible_text.rs` | `InvisibleText` |
| `microscopic_font.rs` | `MicroscopicFont` |
| `text_outside_bounds.rs` | `TextOutsideBounds` |
| `zero_width.rs` | `ZeroWidthChars` |
| `unicode_tricks.rs` | `UnicodeTrick` |
| `token_flooding.rs` | `TokenFlooding` |
| `instruction_patterns.rs` | `InstructionPattern`, `ForeignLanguageInstruction` |
| `javascript.rs` | `EmbeddedJavaScript` |
| `metadata.rs` | `MetadataInjection` |
| `annotations.rs` | `HiddenAnnotation` |
| `forms.rs` | `HiddenFormField` |
| `actual_text.rs` | `ActualTextInjection` |
| `ocg_layer.rs` | `HiddenOcgLayer` |
| `incremental_update.rs` | `IncrementalUpdate` |

## Development Commands

```bash
make test           # Run all unit tests
make test-verbose   # Run tests with output
make dev            # Run app in development mode
make build          # Build debug
make lint           # Run clippy
make fmt            # Format code
```

**Important**: Cargo/Rust is at `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo` (not in default PATH). The Makefile handles this automatically. When running cargo directly:

```bash
cd src-tauri && PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" cargo test --all
```

## Coding Conventions

### Rust

- All detection types must be added to `DetectionType` enum in `models.rs`
- Every new `DetectionType` variant requires corresponding i18n entries in both `pt-BR.json` and `en.json`:
  - `detection_type_<Variant>` — display label
  - `desc_<Variant>` — description shown in findings
  - `info_<Variant>` — tooltip/info panel description
- Report descriptions are translated via `translate_description()` in `report_generator.rs` — add new variants there too
- `AnalysisResult` includes `extracted_text` (full page content) for LLM deep analysis
- Use `regex::Regex::find()` (first match only) to avoid duplicate findings per value
- Prefer returning extracted data (e.g., `Vec<String>`, `Option<String>`) over bare booleans to enable showing excerpts in the UI
- PDF reports must use ASCII-only characters (no Unicode symbols like ✓✗—─) since Arial font lacks these glyphs
- Shared regex patterns in `detectors/mod.rs` use `std::sync::OnceLock<Regex>` — return `&'static Regex`, never recompile per call

### Adding a New Attack Vector (5-step workflow)

1. Add variant to `DetectionType` in `models.rs`
2. Create `src-tauri/src/detectors/my_vector.rs` with a `Signals` struct + `VectorDetector` impl
3. Register in `detectors/mod.rs`: add `pub mod my_vector;` and `Box::new(my_vector::MyVectorDetector),` to `run_all()`
4. Add 3 i18n keys to `src/i18n/pt-BR.json` and `src/i18n/en.json`
5. Add match arm to `report_generator.rs::translate_description()`

### Detector File Template

```rust
use std::any::Any;
use std::collections::HashMap;
use crate::models::{DetectionType, Finding, Severity};
use super::{VectorDetector, SharedSignals};
use lopdf::Document;

pub struct MyVectorDetectorSignals {
    // extracted data from PDF
}

pub struct MyVectorDetector;

impl VectorDetector for MyVectorDetector {
    fn name(&self) -> &'static str { "my_vector" }

    fn extract(&self, doc: &Document, raw_bytes: &[u8], pages: &HashMap<u32, String>) -> Box<dyn Any + Send> {
        // Extract signals from PDF structure
        Box::new(MyVectorDetectorSignals { /* ... */ })
    }

    fn detect(&self, signals: &dyn Any, pages: &HashMap<u32, String>, shared: &SharedSignals) -> Vec<Finding> {
        let signals = signals.downcast_ref::<MyVectorDetectorSignals>().unwrap();
        // Apply heuristic logic, return findings
        vec![]
    }
}
```

### Frontend

- UI language is determined by user settings; all user-visible strings must go through i18n
- `tOptional('excerpt_' + type)` is used for structural findings (when `char_offset == null`)
- Real PDF text excerpts pass through untranslated (they come from the actual PDF content)
- Use Lucide SVG icons (inline) instead of emojis — icon constants are defined in the `ICON` object at top of `app.js`
- Multi-file queue: frontend manages state (`fileQueue` array), processes sequentially, stores `llmResult` per item
- Deep analysis sends `extracted_text` + formatted heuristic findings to the LLM for proper context

### PDF Samples

- Sample attack vectors live in `samples/vectors/` (01–16)
- Each PDF must have valid xref structure: comments BEFORE the `xref` section, never between `startxref` and `%%EOF`
- PDFs should render the legal document visibly; attack content should be hidden via the specific vector technique
- When rebuilding PDFs: use Python scripts with `zlib.compress()`, track byte offsets for xref, format as `f"{offset:010d} 00000 n \n"`

## Detection Type Priority

When multiple structural signals are detected, the primary detection type follows this priority (highest first):

1. `IncrementalUpdate` — multiple %%EOF markers
2. `HiddenOcgLayer` — OCG layer in /OFF array
3. `WhiteText` — color set to (1,1,1)
4. `InvisibleText` — render mode 3
5. `MicroscopicFont` — font size < 2pt
6. `TextOutsideBounds` — coordinates outside MediaBox
7. `InstructionPattern` — generic fallback

Special overrides:
- English instructions → always `ForeignLanguageInstruction`
- Fake citations in hidden text → `CitationPoisoning` (suppresses standalone WhiteText)
- `CitationPoisoning` only fires when `has_white_text` is true (detected in `WhiteTextDetector`)

## Testing

- All tests are in the same files as the code (Rust convention)
- Each detector file has unit tests for its own signals (2–3 per file)
- Integration tests in `detectors/mod.rs` use sample PDFs from `../samples/vectors/`
- Tests guard with `if !path.exists() { return; }` for CI environments without samples
- Always run `make test` (or `cargo test --all`) after changes to verify nothing breaks

## Gotchas

- `lopdf::extract_text()` only reads page content streams, NOT Form XObjects — hidden XObject text must be extracted separately
- `/OC` (Optional Content) only works on Form XObjects, not directly on page content streams
- `lopdf` reads from the END of the file to find `startxref`/`%%EOF` — any content after `%%EOF` breaks parsing
- Python `pip install` doesn't work (Homebrew externally-managed env) — use inline scripts only
- The Tauri binary requires a window; you can't run it as a CLI tool for testing
- `SharedSignals` is populated by a pre-pass in `build_shared_signals()` before detectors run — add new cross-detector flags there if needed
- `InvisibleTextDetector.detect()` skips if `shared.has_incremental_update` (render mode 3 is a known false positive in incremental-update PDFs)
