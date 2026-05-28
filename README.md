# PDF Prompt Injection Evaluator

A desktop application that helps lawyers verify PDF files for prompt injection attacks.

## Features

- **Heuristic Detection (Layer 1)**: Offline scanning for zero-width characters, hidden text, Unicode tricks, embedded JavaScript, suspicious annotations, and instruction patterns (pt-BR + EN)
- **LLM Semantic Analysis (Layer 2)**: Optional deep analysis using OpenAI, Gemini, Anthropic, or custom LLM endpoints
- **PDF Viewer**: In-app PDF rendering with visual highlighting of suspicious regions
- **Report Export**: Generate PDF reports attachable to legal proceedings (includes SHA-256 hash for integrity)
- **Bilingual**: Full support for Portuguese (BR) and English

## Tech Stack

- **Backend**: Rust + Tauri v2
- **Frontend**: HTML/CSS/JS + PDF.js
- **PDF Parsing**: lopdf + pdf-extract
- **Report Generation**: genpdf

## Development

### Prerequisites

- Rust (latest stable)
- Cargo
- Tauri CLI v2: `cargo install tauri-cli --version "^2"`

### Run in development

```bash
cargo tauri dev
```

### Build for production

```bash
cargo tauri build
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
Sends suspicious excerpts to your configured LLM for semantic classification of injection attempts.

## License

MIT
