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
| `src-tauri/src/models.rs` | Data types: `DetectionType` enum, `Finding`, `Severity`, `AnalysisResult` |
| `src-tauri/src/pdf_parser.rs` | PDF parsing: text extraction, structural signal detection (white text, OCG, JS, forms) |
| `src-tauri/src/heuristic_detector.rs` | Detection logic: maps structural signals + regex patterns → findings |
| `src-tauri/src/llm_analyzer.rs` | Optional LLM-based semantic analysis (Layer 2) |
| `src-tauri/src/report_generator.rs` | PDF report generation for legal proceedings |
| `src/app.js` | Frontend: UI rendering, i18n, file upload handling |
| `src/i18n/pt-BR.json` | Portuguese translations (primary language) |
| `src/i18n/en.json` | English translations |

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
- The `make_content()` test helper in `heuristic_detector.rs` must include all `PdfContent` fields
- Use `regex::Regex::find()` (first match only) to avoid duplicate findings per value
- Prefer returning extracted data (e.g., `Vec<String>`, `Option<String>`) over bare booleans to enable showing excerpts in the UI

### Frontend

- UI language is determined by user settings; all user-visible strings must go through i18n
- `tOptional('excerpt_' + type)` is used for structural findings (when `char_offset == null`)
- Real PDF text excerpts pass through untranslated (they come from the actual PDF content)

### PDF Samples

- Sample attack vectors live in `samples/vectors/` (01–15)
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
- `CitationPoisoning` only fires when `has_white_text || has_microscopic_font`

## Testing

- All tests are in the same files as the code (Rust convention)
- Integration tests use sample PDFs from `../samples/vectors/`
- Tests guard with `if !path.exists() { return; }` for CI environments without samples
- Always run `make test` (or `cargo test --all`) after changes to verify nothing breaks

## Gotchas

- `lopdf::extract_text()` only reads page content streams, NOT Form XObjects — hidden XObject text must be extracted separately
- `/OC` (Optional Content) only works on Form XObjects, not directly on page content streams
- `lopdf` reads from the END of the file to find `startxref`/`%%EOF` — any content after `%%EOF` breaks parsing
- Python `pip install` doesn't work (Homebrew externally-managed env) — use inline scripts only
- The Tauri binary requires a window; you can't run it as a CLI tool for testing
