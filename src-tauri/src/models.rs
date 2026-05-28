use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    Warning,
    Clean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionType {
    ZeroWidthChars,
    InvisibleText,
    WhiteText,
    MicroscopicFont,
    TextOutsideBounds,
    HiddenAnnotation,
    HiddenFormField,
    InstructionPattern,
    UnicodeTrick,
    EmbeddedJavaScript,
    MetadataInjection,
    TokenFlooding,
    ForeignLanguageInstruction,
    IncrementalUpdate,
    ActualTextInjection,
    HiddenOcgLayer,
    CitationPoisoning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub page: u32,
    pub severity: Severity,
    pub detection_type: DetectionType,
    pub description: String,
    pub excerpt: String,
    pub char_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Verdict {
    Safe,
    Unsafe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub file_hash: String,
    pub filename: String,
    pub analyzed_at: String,
    pub extracted_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassification {
    pub confidence: u8,
    pub classification: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Gemini,
    Anthropic,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub custom_endpoint: Option<String>,
    pub language: String,
}
