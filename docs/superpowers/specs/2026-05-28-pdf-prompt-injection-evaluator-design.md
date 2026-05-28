# PDF Prompt Injection Evaluator — Design Spec

## Overview

A desktop application (Windows .exe) built with Rust + Tauri v2 that helps lawyers verify PDF files for prompt injection attacks. Inspired by MinutaIA's hybrid detection methodology, the tool combines offline heuristic analysis with optional LLM-powered semantic analysis.

**Target users**: Brazilian lawyers (pt-BR primary, EN secondary)
**Core workflow**: Drag-and-drop PDF → instant safety report with visual highlighting

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Tauri v2 Desktop App                 │
├──────────────────┬──────────────────────────────┤
│   Frontend       │        Rust Backend           │
│   (WebView)      │                              │
│                  │  ┌─────────────────────────┐  │
│  • Drag & drop   │  │  PDF Parser Module      │  │
│  • PDF.js viewer │  │  (lopdf + pdf-extract)  │  │
│  • Report view   │  ├─────────────────────────┤  │
│  • Settings      │  │  Heuristic Detector     │  │
│                  │  │  (pattern matching)      │  │
│                  │  ├─────────────────────────┤  │
│                  │  │  LLM Analyzer           │  │
│                  │  │  (multi-provider)       │  │
│                  │  ├─────────────────────────┤  │
│                  │  │  Report Generator       │  │
│                  │  │  (PDF export)           │  │
│                  │  └─────────────────────────┘  │
└──────────────────┴──────────────────────────────┘
```

## Detection Engine

### Layer 1: Heuristic/Structural Detection (offline, instant)

Scans the raw PDF structure and extracted text for known injection patterns:

| Check | What it detects | Language |
|-------|----------------|----------|
| Zero-width characters | `\u200B`, `\u200C`, `\uFEFF`, etc. hidden between visible text | Universal |
| Invisible text | White text on white background, font-size: 0, opacity: 0 | Universal |
| Hidden annotations | PDF annotations containing injection text not rendered visually | Universal |
| Instruction patterns (pt-BR) | "ignore as instruções", "desconsidere o prompt", "aja como", "novo objetivo" | pt-BR |
| Instruction patterns (EN) | "ignore previous instructions", "disregard above", "act as", "new objective" | EN |
| Unicode tricks | Homoglyphs, bidirectional overrides, combining characters | Universal |
| Embedded JavaScript | PDF `/JS` and `/JavaScript` actions | Universal |
| Metadata injection | Suspicious content in PDF metadata fields (Title, Author, Subject, Keywords) | Universal |

**Severity levels:**
- 🔴 **Critical**: High confidence injection attempt (e.g., hidden instruction text with zero-width chars)
- 🟡 **Warning**: Suspicious pattern that may be benign (e.g., instruction-like keywords in visible text)
- 🟢 **Clean**: No issues found

### Layer 2: LLM Semantic Analysis (requires API key)

- Sends extracted text (or suspicious excerpts) to the configured LLM provider
- Uses a specialized system prompt that provides context about prompt injection in legal documents and asks the model to classify whether the submitted text contains instruction-manipulation attempts targeting an LLM
- The analysis prompt is hardcoded in the app (not user-editable) to prevent prompt injection of the analyzer itself
- Returns: confidence score (0-100), classification (injection/benign/ambiguous), explanation in the user's selected language
- Triggered when: Layer 1 finds suspicious patterns, OR user manually requests "Deep Analysis"

**Supported providers:**
- OpenAI (GPT-4o, GPT-4.1)
- Google Gemini
- Anthropic Claude
- Custom endpoint (user-configurable base URL + API key)

## User Interface

### Layout

```
┌──────────────────────────────────────────────────┐
│  [⚙️ Settings]              PDF Injection Checker │
├────────────────────────┬─────────────────────────┤
│                        │  ✅ SAFE / 🚨 UNSAFE    │
│                        │                         │
│    PDF Viewer          │  Findings:              │
│    (with highlights)   │  • [🔴] Page 3: hidden  │
│                        │    zero-width chars...   │
│                        │  • [🟡] Page 7: text    │
│                        │    "ignore instruções"   │
│                        │                         │
│                        │  [🔍 Deep Analysis]     │
│                        │  [📄 Export Report]     │
├────────────────────────┴─────────────────────────┤
│  Drop a PDF here or click to browse              │
└──────────────────────────────────────────────────┘
```

### Workflow

1. **Drop zone**: Lawyer drags PDF into the app (or clicks to browse)
2. **Scanning**: Brief progress indicator while Layer 1 heuristic analysis runs
3. **Results panel** (right): Shows pass/fail verdict with ordered findings list
4. **PDF viewer** (left): Renders the PDF using PDF.js, highlighting suspicious regions
5. **Deep Analysis button**: Triggers Layer 2 LLM analysis (only if API key configured)
6. **Export Report**: Generates a PDF document summarizing all findings

### Settings Page

- LLM provider dropdown (OpenAI / Gemini / Anthropic / Custom)
- API key input (stored securely in OS keychain via Tauri's secure storage)
- Language preference: pt-BR / EN / Auto-detect from PDF content
- Custom endpoint URL (for self-hosted models)

## Report Export

The exported PDF report contains:

- **Header**: App name, analysis date and time, filename analyzed
- **Summary**: Overall verdict (Safe/Unsafe), aggregate risk score
- **Findings table**: Each finding with page number, detection type, severity, text excerpt
- **LLM analysis section** (if Deep Analysis was run): Semantic classification results with explanations
- **Integrity proof**: SHA-256 hash of the original PDF file
- **Footer**: Tool version, disclaimer

This report is designed to be attachable to legal proceedings as evidence of due diligence.

## Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| App framework | Tauri v2 | Desktop app with small binary, native performance |
| Backend language | Rust | PDF parsing, detection logic, report generation |
| PDF parsing | `lopdf` + `pdf-extract` | Raw PDF structure access + text extraction |
| Pattern matching | `regex` crate | Heuristic detection rules |
| HTTP client | `reqwest` (async) | LLM API calls |
| Frontend | HTML + CSS + vanilla JS | Minimal UI, fast load |
| PDF viewer | PDF.js | In-app PDF rendering with highlight annotations |
| Report generation | `genpdf` or `printpdf` | Create exportable PDF reports |
| i18n | JSON translation files | pt-BR and EN support |
| Secure storage | Tauri secure store plugin | API key persistence |
| Build target | Windows .exe via `cargo tauri build` | Distribution |

## File Structure

```
pdf-prompt-injection-evaluator/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app entry point, command handlers
│   │   ├── pdf_parser.rs        # PDF text/structure extraction
│   │   ├── heuristic_detector.rs # Pattern-based injection detection
│   │   ├── llm_analyzer.rs      # Multi-provider LLM API integration
│   │   ├── report_generator.rs  # PDF report creation
│   │   └── config.rs            # App configuration and settings
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                          # Frontend (WebView)
│   ├── index.html
│   ├── styles.css
│   ├── app.js                   # Main app logic, Tauri IPC
│   ├── pdf-viewer.js            # PDF.js integration + highlighting
│   └── i18n/
│       ├── pt-BR.json
│       └── en.json
├── docs/
│   └── superpowers/specs/       # This spec
└── README.md
```

## Internationalization

- UI labels and messages available in pt-BR and EN
- Detection patterns include both Portuguese and English injection phrases
- Auto-detect language from PDF content to prioritize relevant patterns
- User can override language in settings

## Security Considerations

- API keys stored in OS keychain (not plaintext config files)
- PDF parsing runs in sandboxed Rust — no code execution from PDF content
- LLM requests send only text excerpts, never the full PDF binary
- No telemetry or data collection — fully offline-capable (Layer 1)

## Out of Scope (v1)

- macOS/Linux builds (Windows .exe only for v1)
- PDF sanitization/cleaning
- Batch processing of multiple PDFs
- Cloud/server deployment
- User accounts or license management
