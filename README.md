# PDF Prompt Injection Evaluator

A desktop application that helps lawyers verify PDF files for prompt injection attacks.

## Features

- **Heuristic Detection (Layer 1)**: Offline scanning for zero-width characters, hidden text, Unicode tricks, embedded JavaScript, suspicious annotations, and instruction patterns (pt-BR + EN)
- **LLM Semantic Analysis (Layer 2)**: Optional deep analysis using OpenAI, Gemini, Anthropic, or custom LLM endpoints
- **Multi-file Queue**: Process multiple PDFs at once with progress tracking and batch deep analysis
- **PDF Viewer**: In-app PDF rendering with visual highlighting of suspicious regions
- **Report Export**: Generate PDF/text reports (single or batch) attachable to legal proceedings (includes SHA-256 hash for integrity)
- **Bilingual**: Full support for Portuguese (BR) and English — including report generation
- **Persistent Settings**: Saves language, LLM provider, and API key across sessions
- **Lucide Icons**: Clean SVG icon set for all UI indicators

## Tech Stack

- **Backend**: Rust + Tauri v2
- **Frontend**: HTML/CSS/JS + PDF.js
- **PDF Parsing**: lopdf + pdf-extract
- **Report Generation**: genpdf

## Development

### Prerequisites

- Rust (latest stable)
- Cargo
- Tauri CLI v2

### Setup

```bash
make install-deps
```

### Available Commands

| Command | Description |
|---------|-------------|
| `make help` | Show all available targets |
| `make dev` | Run the app in development mode |
| `make build` | Build the Rust backend (debug) |
| `make release` | Build production release for current platform |
| `make release-mac` | Build macOS bundle (.app + .dmg) for Apple Silicon |
| `make release-win` | Build Windows bundle (.exe / .msi) |
| `make test` | Run all unit tests |
| `make test-verbose` | Run tests with output |
| `make lint` | Run clippy linter |
| `make fmt` | Format Rust code |
| `make fmt-check` | Check formatting without modifying files |
| `make check` | Fast compile verification |
| `make clean` | Remove build artifacts |
| `make update` | Update Rust dependencies |

### Quick Start

```bash
make install-deps   # one-time setup
make dev            # run in development
```

### Build for Production

```bash
make release
```

The executable will be in `src-tauri/target/release/bundle/`.

## Configuration

Click the ⚙️ button to configure:
- LLM provider and API key
- Language preference (pt-BR / EN)
- Custom endpoint URL (for self-hosted models)

## How It Works

### Layer 1: Heuristic Detection (offline)
Scans the PDF structure for:
- Zero-width characters hiding text
- Bidirectional Unicode overrides
- Instruction manipulation patterns (pt-BR and EN)
- Embedded JavaScript
- Suspicious metadata and annotations

### Layer 2: LLM Semantic Analysis (optional)
Sends the full extracted text plus heuristic findings to your configured LLM for semantic classification of injection attempts. Works on individual files or in batch mode across the entire queue.

## Test Samples

The `samples/` directory contains PDF files you can use to test the application and understand different injection techniques:

### Basic Samples

| File | Description |
|------|-------------|
| `peticao_limpa.pdf` | Clean labor petition — no injection (control file) |
| `peticao_com_injection.pdf` | Same petition with white-text injection hidden before visible content |

### Injection Vector Samples (`samples/vectors/`)

One PDF per attack vector, based on the [15 documented techniques](https://www.migalhas.com.br/depeso/455924/prompt-injection-em-documentos-judiciais-conceito-vetores-e-riscos) for PDF prompt injection in judicial documents:

| # | File | Vector | Sophistication |
|---|------|--------|---------------|
| 01 | `01_texto_branco.pdf` | White text on white background | Minimal |
| 02 | `02_metadados_info.pdf` | Injection in /Info metadata (Subject, Keywords, Creator) | Minimal |
| 03 | `03_texto_fora_limites.pdf` | Text positioned outside visible page boundaries | Low |
| 04 | `04_fonte_microscopica.pdf` | Microscopic font size (<2pt) | Low |
| 05 | `05_unicode_invisivel.pdf` | Zero-width spaces (U+200B), bidi override (U+202E) | Low |
| 06 | `06_revisao_incremental.pdf` | Content appended after digital signature (incremental update) | Medium |
| 07 | `07_actualtext.pdf` | /ActualText accessibility attribute with divergent content | Medium |
| 08 | `08_campos_assinatura.pdf` | Signature fields (/Reason, /Location) with directives | Medium |
| 09 | `09_acroform_oculto.pdf` | Hidden AcroForm fields with injection values | Medium |
| 10 | `10_camada_ocg_off.pdf` | Disabled OCG layer (invisible but extractable) | Medium |
| 11 | `11_idioma_estrangeiro.pdf` | English instructions in Portuguese document | Low/Medium |
| 12 | `12_token_flooding.pdf` | Massive repetition of favorable legal terms | High |
| 13 | `13_tounicode_cmap.pdf` | Tampered font-to-Unicode mapping (extracted ≠ visible) | High |
| 14 | `14_citation_poisoning.pdf` | Fake jurisprudence and non-existent legal precedents | High |
| 15 | `15_javascript_openaction.pdf` | Embedded JavaScript via /OpenAction | High |
| 16 | `16_instrucao_visivel.pdf` | Visible prompt injection buried in contract text | Minimal |

### How to Use

1. Open the application with `make dev`
2. Drag and drop any sample PDF into the drop zone
3. Compare results between the clean file and the injected ones
4. The heuristic detector should flag vectors 01–05, 11–12, 15–16 automatically
5. For advanced vectors (06–10, 13–14), use the LLM deep analysis for better detection

> **Reference:** These vectors are documented in [this article](https://www.migalhas.com.br/depeso/455924/prompt-injection-em-documentos-judiciais-conceito-vetores-e-riscos) about the first judicial conviction for prompt injection in Brazil (Parauapebas/PA, May 2026).

## License

MIT
