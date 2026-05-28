import { renderPdfFromBytes, getPageCount, goToPage } from './pdf-viewer.js';

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

let currentResult = null;
let currentLlmResult = null;
let currentFilePath = null;

// DOM elements
const dropZoneContainer = document.getElementById('drop-zone-container');
const dropZone = document.getElementById('drop-zone');
const analysisView = document.getElementById('analysis-view');
const verdictBanner = document.getElementById('verdict-banner');
const findingsList = document.getElementById('findings-list');
const loadingOverlay = document.getElementById('loading-overlay');
const settingsModal = document.getElementById('settings-modal');
const settingsBtn = document.getElementById('settings-btn');
const saveSettingsBtn = document.getElementById('save-settings-btn');
const cancelSettingsBtn = document.getElementById('cancel-settings-btn');
const deepAnalysisBtn = document.getElementById('deep-analysis-btn');
const exportBtn = document.getElementById('export-btn');
const newFileBtn = document.getElementById('new-file-btn');
const providerSelect = document.getElementById('provider-select');
const apiKeyInput = document.getElementById('api-key-input');
const customEndpoint = document.getElementById('custom-endpoint');
const endpointLabel = document.getElementById('endpoint-label');
const languageSelect = document.getElementById('language-select');

// File selection via Tauri dialog
async function selectFile() {
    const selected = await open({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (selected) {
        await analyzePdfFile(selected);
    }
}

// Drop zone events
dropZone.addEventListener('click', selectFile);
dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('dragover');
});
dropZone.addEventListener('dragleave', () => dropZone.classList.remove('dragover'));
dropZone.addEventListener('drop', async (e) => {
    e.preventDefault();
    dropZone.classList.remove('dragover');
    // Tauri drag-drop provides file paths differently
    // For now, use the dialog approach
    await selectFile();
});

async function analyzePdfFile(path) {
    currentFilePath = path;
    showLoading(true);

    try {
        currentResult = await invoke('analyze_pdf', { path });
        showResults();
    } catch (err) {
        alert('Error: ' + err);
    } finally {
        showLoading(false);
    }
}

function showResults() {
    dropZoneContainer.classList.add('hidden');
    analysisView.classList.remove('hidden');

    // Verdict
    if (currentResult.verdict === 'Safe') {
        verdictBanner.textContent = '✅ SAFE — No injection detected';
        verdictBanner.className = 'safe';
    } else {
        verdictBanner.textContent = '🚨 UNSAFE — Potential injection detected';
        verdictBanner.className = 'unsafe';
    }

    // Findings
    findingsList.innerHTML = '';
    if (currentResult.findings.length === 0) {
        findingsList.innerHTML = '<p style="color: var(--text-muted)">No findings.</p>';
    } else {
        for (const finding of currentResult.findings) {
            const severity = finding.severity === 'Critical' ? 'critical' : 'warning';
            const icon = finding.severity === 'Critical' ? '🔴' : '🟡';
            const el = document.createElement('div');
            el.className = 'finding-item ' + severity;
            el.innerHTML =
                '<div class="finding-header">' + icon + ' Page ' + finding.page + ': ' + escapeHtml(finding.description) + '</div>' +
                (finding.excerpt ? '<div class="finding-excerpt">"' + escapeHtml(finding.excerpt) + '"</div>' : '');
            findingsList.appendChild(el);
        }
    }

    renderPdf(currentFilePath);
}

async function renderPdf(path) {
    try {
        const { readFile } = window.__TAURI__.fs;
        const fileBytes = await readFile(path);
        await renderPdfFromBytes(fileBytes, currentResult ? currentResult.findings : []);
    } catch (err) {
        console.error('PDF render failed:', err);
        const canvas = document.getElementById('pdf-canvas');
        canvas.width = 400;
        canvas.height = 560;
        const ctx = canvas.getContext('2d');
        ctx.fillStyle = '#222';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#888';
        ctx.font = '14px sans-serif';
        ctx.fillText('PDF preview unavailable', 20, 40);
    }
}

// Deep Analysis
deepAnalysisBtn.addEventListener('click', async () => {
    if (!currentResult) return;

    showLoading(true);
    try {
        const allText = currentResult.findings.map(f => f.excerpt).filter(Boolean).join('\n');
        currentLlmResult = await invoke('deep_analysis', { text: allText || 'No suspicious text found' });
        displayLlmResult();
    } catch (err) {
        alert('LLM Analysis error: ' + err);
    } finally {
        showLoading(false);
    }
});

function displayLlmResult() {
    if (!currentLlmResult) return;
    const existing = document.getElementById('llm-result');
    if (existing) existing.remove();

    const el = document.createElement('div');
    el.id = 'llm-result';
    el.className = 'finding-item';
    el.style.borderLeftColor = currentLlmResult.classification === 'injection' ? 'var(--accent)' : 'var(--accent-green)';
    el.innerHTML =
        '<div class="finding-header">🤖 LLM Analysis: ' + escapeHtml(currentLlmResult.classification) + ' (' + currentLlmResult.confidence + '% confidence)</div>' +
        '<div class="finding-excerpt">' + escapeHtml(currentLlmResult.explanation) + '</div>';
    findingsList.appendChild(el);
}

// Export
exportBtn.addEventListener('click', async () => {
    if (!currentResult) return;
    const outputPath = await save({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        defaultPath: 'report-' + currentResult.filename,
    });
    if (outputPath) {
        try {
            await invoke('export_report', {
                result: currentResult,
                llmResult: currentLlmResult,
                outputPath,
            });
            alert('Report exported successfully!');
        } catch (err) {
            alert('Export error: ' + err);
        }
    }
});

// New File
newFileBtn.addEventListener('click', () => {
    currentResult = null;
    currentLlmResult = null;
    currentFilePath = null;
    analysisView.classList.add('hidden');
    dropZoneContainer.classList.remove('hidden');
});

// Settings
settingsBtn.addEventListener('click', async () => {
    const config = await invoke('get_config');
    providerSelect.value = config.provider;
    apiKeyInput.value = config.api_key;
    languageSelect.value = config.language;
    if (config.custom_endpoint) customEndpoint.value = config.custom_endpoint;
    toggleCustomEndpoint();
    settingsModal.classList.remove('hidden');
});

cancelSettingsBtn.addEventListener('click', () => settingsModal.classList.add('hidden'));

saveSettingsBtn.addEventListener('click', async () => {
    const config = {
        provider: providerSelect.value,
        api_key: apiKeyInput.value,
        custom_endpoint: providerSelect.value === 'Custom' ? customEndpoint.value : null,
        language: languageSelect.value,
    };
    try {
        await invoke('save_settings', { config });
        settingsModal.classList.add('hidden');
    } catch (err) {
        alert('Save error: ' + err);
    }
});

providerSelect.addEventListener('change', toggleCustomEndpoint);

function toggleCustomEndpoint() {
    const show = providerSelect.value === 'Custom';
    customEndpoint.classList.toggle('hidden', !show);
    endpointLabel.classList.toggle('hidden', !show);
}

function showLoading(show) {
    loadingOverlay.classList.toggle('hidden', !show);
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}
