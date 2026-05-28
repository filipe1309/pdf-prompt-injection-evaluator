import { renderPdfFromBytes, getPageCount, goToPage } from './pdf-viewer.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open, save } = window.__TAURI__.dialog;

let translations = {};

async function loadTranslations() {
    try {
        const config = await invoke('get_config');
        const lang = config.language || 'pt-BR';
        const resp = await fetch('./i18n/' + lang + '.json');
        translations = await resp.json();
        document.documentElement.lang = lang;
        applyTranslations();
    } catch (e) {
        console.warn('Failed to load translations, using defaults');
    }
}

function t(key) {
    return translations[key] || key;
}

function tOptional(key) {
    return translations[key] || null;
}

function applyTranslations() {
    const el = (id) => document.getElementById(id);
    if (el('app-title')) el('app-title').textContent = t('app_title');
    if (el('drop-text-primary')) el('drop-text-primary').textContent = t('drop_primary');
    if (el('drop-text-secondary')) el('drop-text-secondary').textContent = translations.drop_secondary ?? '';
    if (el('settings-title')) el('settings-title').textContent = t('settings');
    if (el('provider-label')) el('provider-label').textContent = t('provider_label');
    if (el('apikey-label')) el('apikey-label').textContent = t('apikey_label');
    if (el('endpoint-label')) el('endpoint-label').textContent = t('endpoint_label');
    if (el('language-label')) el('language-label').textContent = t('language_label');
    if (el('save-settings-btn')) el('save-settings-btn').textContent = t('save');
    if (el('cancel-settings-btn')) el('cancel-settings-btn').textContent = t('cancel');
    if (el('deep-analysis-btn')) el('deep-analysis-btn').title = t('deep_analysis');
    if (el('export-btn')) el('export-btn').title = t('export_report');
    if (el('new-file-btn')) el('new-file-btn').title = t('new_file');
    if (el('loading-text')) el('loading-text').textContent = t('analyzing');
    document.title = t('app_title');
}

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
const headerActions = document.getElementById('header-actions');
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
dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('dragover');
});

// Tauri v2 drag-and-drop: listen for file drop events from the OS
listen('tauri://drag-drop', async (event) => {
    const paths = event.payload?.paths;
    if (paths && paths.length > 0) {
        const pdfPath = paths.find(p => p.toLowerCase().endsWith('.pdf'));
        if (pdfPath) {
            await analyzePdfFile(pdfPath);
        }
    }
});

listen('tauri://drag-enter', () => {
    dropZone.classList.add('dragover');
});

listen('tauri://drag-leave', () => {
    dropZone.classList.remove('dragover');
});

async function analyzePdfFile(path) {
    currentFilePath = path;
    showLoading(true);

    try {
        const startTime = Date.now();
        currentResult = await invoke('analyze_pdf', { path });
        // Ensure loading is visible for at least 800ms for UX feedback
        const elapsed = Date.now() - startTime;
        if (elapsed < 800) {
            await new Promise(r => setTimeout(r, 800 - elapsed));
        }
        showResults();
    } catch (err) {
        console.error('[app] Analysis error:', err);
        alert(t('error') + ': ' + err);
    } finally {
        showLoading(false);
    }
}

function showResults() {
    dropZoneContainer.classList.add('hidden');
    analysisView.classList.remove('hidden');
    headerActions.classList.remove('hidden');

    // Verdict
    if (currentResult.verdict === 'Safe') {
        verdictBanner.textContent = t('safe');
        verdictBanner.className = 'safe';
    } else {
        verdictBanner.textContent = t('unsafe');
        verdictBanner.className = 'unsafe';
    }

    // Findings
    findingsList.innerHTML = '';
    if (currentResult.findings.length === 0) {
        findingsList.innerHTML = '<p style="color: var(--text-muted)">' + t('no_findings') + '</p>';
    } else {
        for (const finding of currentResult.findings) {
            const severity = finding.severity === 'Critical' ? 'critical' : 'warning';
            const icon = finding.severity === 'Critical' ? '🔴' : '🟡';
            const typeTag = t('detection_type_' + finding.detection_type) || finding.detection_type;
            const description = t('desc_' + finding.detection_type) || finding.description;
            const el = document.createElement('div');
            el.className = 'finding-item ' + severity;
                const excerptText = finding.char_offset == null
                    ? (tOptional('excerpt_' + finding.detection_type) || finding.excerpt)
                    : finding.excerpt;
                el.innerHTML =
                '<div class="finding-header">' +
                    '<span class="finding-tag tag-' + severity + '">' + escapeHtml(typeTag) + '</span> ' +
                    icon + ' ' + (finding.page > 0 ? t('page') + ' ' + finding.page + ': ' : '') + escapeHtml(description) +
                '</div>' +
                (finding.excerpt ? '<div class="finding-excerpt">"' + escapeHtml(excerptText) + '"</div>' : '');
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
            alert(t('export_success'));
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
    headerActions.classList.add('hidden');
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
        await loadTranslations();
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

// Initialize translations on load
loadTranslations();

// Info modal
const infoBtn = document.getElementById('info-btn');
const infoModal = document.getElementById('info-modal');
const closeInfoBtn = document.getElementById('close-info-btn');
const infoList = document.getElementById('info-list');

const INJECTION_TYPES = [
    { type: 'WhiteText', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/></svg>' },
    { type: 'InvisibleText', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/></svg>' },
    { type: 'MicroscopicFont', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="M11 8v6"/><path d="M8 11h6"/></svg>' },
    { type: 'TextOutsideBounds', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 3 3 21"/><path d="M21 3H8"/><path d="M21 3v13"/></svg>' },
    { type: 'HiddenOcgLayer', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m2 12 8.58 3.91a2 2 0 0 0 1.66 0L21 12"/><path d="m2 17 8.58 3.91a2 2 0 0 0 1.66 0L21 17"/></svg>' },
    { type: 'IncrementalUpdate', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M12 18v-6"/><path d="m9 15 3-3 3 3"/></svg>' },
    { type: 'ActualTextInjection', severity: 'critical', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2H2v10l9.29 9.29c.94.94 2.48.94 3.42 0l6.58-6.58c.94-.94.94-2.48 0-3.42L12 2Z"/><path d="M7 7h.01"/></svg>' },
    { type: 'EmbeddedJavaScript', severity: 'critical', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 16 4-4-4-4"/><path d="m6 8-4 4 4 4"/><path d="m14.5 4-5 16"/></svg>' },
    { type: 'HiddenAnnotation', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z"/></svg>' },
    { type: 'HiddenFormField', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 7h10"/><path d="M7 12h10"/><path d="M7 17h10"/></svg>' },
    { type: 'ForeignLanguageInstruction', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>' },
    { type: 'TokenFlooding', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 6c.6.5 1.2 1 2.5 1C7 7 7 5 9.5 5c2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/><path d="M2 12c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/><path d="M2 18c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/></svg>' },
    { type: 'CitationPoisoning', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/></svg>' },
    { type: 'MetadataInjection', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 7h-9"/><path d="M14 17H5"/><circle cx="17" cy="17" r="3"/><circle cx="7" cy="7" r="3"/></svg>' },
    { type: 'InstructionPattern', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>' },
    { type: 'ZeroWidthChars', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7V4h16v3"/><path d="M9 20h6"/><path d="M12 4v16"/></svg>' },
    { type: 'UnicodeTrick', severity: 'warning', icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m8 3 4 8 5-5 5 15H2L8 3z"/></svg>' },
];

infoBtn.addEventListener('click', () => {
    renderInfoModal();
    infoModal.classList.remove('hidden');
});

closeInfoBtn.addEventListener('click', () => infoModal.classList.add('hidden'));
infoModal.addEventListener('click', (e) => {
    if (e.target === infoModal) infoModal.classList.add('hidden');
});

function renderInfoModal() {
    if (document.getElementById('info-title')) {
        document.getElementById('info-title').textContent = t('info_title');
    }
    infoList.innerHTML = INJECTION_TYPES.map(({ type, severity, icon }) => {
        const title = t('detection_type_' + type);
        const desc = t('info_' + type);
        const sevLabel = severity === 'critical' ? t('info_severity_critical') : t('info_severity_warning');
        return `
            <div class="info-item ${severity}">
                <div class="info-item-title">
                    <span>${icon}</span>
                    <span>${escapeHtml(title)}</span>
                    <span class="info-item-severity ${severity}">${sevLabel}</span>
                </div>
                <div class="info-item-desc">${escapeHtml(desc)}</div>
            </div>
        `;
    }).join('');
}
