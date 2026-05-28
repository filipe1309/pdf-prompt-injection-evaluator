#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pdf_prompt_injection_evaluator_lib::{
    config,
    heuristic_detector,
    llm_analyzer,
    models::{AnalysisResult, AppConfig, LlmClassification, Severity, Verdict},
    pdf_parser,
    report_generator,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[tauri::command]
async fn analyze_pdf(path: String) -> Result<AnalysisResult, String> {
    let file_path = PathBuf::from(&path);

    let file_bytes = std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let hash = format!("{:x}", Sha256::digest(&file_bytes));

    let content = pdf_parser::parse_pdf(&file_path).map_err(|e| format!("PDF parse error: {}", e))?;

    let findings = heuristic_detector::detect(&content);

    let verdict = if findings
        .iter()
        .any(|f| f.severity == Severity::Critical || f.severity == Severity::Warning)
    {
        Verdict::Unsafe
    } else {
        Verdict::Safe
    };

    let filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(AnalysisResult {
        verdict,
        findings,
        file_hash: hash,
        filename,
        analyzed_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

#[tauri::command]
async fn deep_analysis(text: String) -> Result<LlmClassification, String> {
    let cfg = config::load_config();
    llm_analyzer::analyze(&text, &cfg)
        .await
        .map_err(|e| format!("LLM error: {}", e))
}

#[tauri::command]
fn get_config() -> AppConfig {
    config::load_config()
}

#[tauri::command]
fn save_settings(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
async fn export_report(
    result: AnalysisResult,
    llm_result: Option<LlmClassification>,
    output_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(output_path);
    report_generator::generate_report(&result, llm_result.as_ref(), &path)
        .map_err(|e| format!("Report error: {}", e))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            analyze_pdf,
            deep_analysis,
            get_config,
            save_settings,
            export_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
