import { renderPdfFromBytes, getPageCount, goToPage } from './pdf-viewer.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open, save } = window.__TAURI__.dialog;
const { getVersion } = window.__TAURI__.app;

// Lucide icon SVGs (inline, 16x16)
const ICON = {
    shieldCheck: '<svg class="icon icon-safe" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></svg>',
    shieldAlert: '<svg class="icon icon-unsafe" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="M12 8v4"/><path d="M12 16h.01"/></svg>',
    circleAlert: '<svg class="icon icon-critical" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4"/><path d="M12 16h.01"/></svg>',
    triangleAlert: '<svg class="icon icon-warning" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>',
    bot: '<svg class="icon icon-bot" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/></svg>',
    clock: '<svg class="icon icon-pending" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    loader: '<svg class="icon icon-processing" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v4"/><path d="m16.2 7.8 2.9-2.9"/><path d="M18 12h4"/><path d="m16.2 16.2 2.9 2.9"/><path d="M12 18v4"/><path d="m4.9 19.1 2.9-2.9"/><path d="M2 12h4"/><path d="m4.9 4.9 2.9 2.9"/></svg>',
    xCircle: '<svg class="icon icon-error" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>',
};

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
    if (el('drop-text-hint')) el('drop-text-hint').textContent = t('drop_hint');
    if (el('settings-title')) el('settings-title').textContent = t('settings');
    if (el('provider-label')) el('provider-label').textContent = t('provider_label');
    if (el('apikey-label')) el('apikey-label').textContent = t('apikey_label');
    if (el('endpoint-label')) el('endpoint-label').textContent = t('endpoint_label');
    if (el('language-label')) el('language-label').textContent = t('language_label');
    if (el('save-settings-btn')) el('save-settings-btn').textContent = t('save');
    if (el('cancel-settings-btn')) el('cancel-settings-btn').textContent = t('cancel');
    if (el('loading-text')) el('loading-text').textContent = t('analyzing');
    document.title = t('app_title');

    // Tooltips from i18n
    document.querySelectorAll('[data-tooltip-key]').forEach(btn => {
        const key = btn.dataset.tooltipKey;
        btn.setAttribute('data-tooltip', t(key));
    });

    // Button labels from i18n
    document.querySelectorAll('[data-i18n]').forEach(el => {
        el.textContent = t(el.dataset.i18n);
    });
}

let currentResult = null;
let currentLlmResult = null;
let currentFilePath = null;
let currentPdfPage = 1;

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
const backToQueueBtn = document.getElementById('back-to-queue-btn');
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
        multiple: true,
    });
    if (selected) {
        const files = Array.isArray(selected) ? selected : [selected];
        if (files.length > 0) {
            await processFileQueue(files);
        }
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
        const pdfPaths = paths.filter(p => p.toLowerCase().endsWith('.pdf'));
        if (pdfPaths.length > 0) {
            await processFileQueue(pdfPaths);
        }
    }
});

listen('tauri://drag-enter', () => {
    dropZone.classList.add('dragover');
});

listen('tauri://drag-leave', () => {
    dropZone.classList.remove('dragover');
});

// Multi-file queue
let fileQueue = [];
let queueResults = [];
let queueFilter = null; // null | 'safe' | 'preliminary' | 'unsafe' | 'pending'

async function processFileQueue(paths) {
    if (paths.length === 1) {
        await analyzePdfFile(paths[0]);
        return;
    }

    fileQueue = paths.map(p => ({ path: p, name: p.split('/').pop().split('\\').pop(), status: 'pending', result: null, fileSize: null }));
    queueResults = [];

    // Get file sizes (best-effort, won't block analysis if permission missing)
    try {
        const { stat } = window.__TAURI__.fs;
        if (stat) {
            for (let i = 0; i < fileQueue.length; i++) {
                try {
                    const meta = await stat(fileQueue[i].path);
                    fileQueue[i].fileSize = meta.size;
                } catch (_) { /* ignore individual file errors */ }
            }
        }
    } catch (_) { /* stat not available */ }

    showQueueView();
    showLoading(true);
    document.getElementById('loading-text').textContent = t('analyzing') || 'Analyzing...';
    const queueStartTime = Date.now();

    for (let i = 0; i < fileQueue.length; i++) {
        fileQueue[i].status = 'processing';
        document.getElementById('loading-text').textContent =
            (t('analyzing') || 'Analyzing...') + ' (' + (i + 1) + '/' + fileQueue.length + ')';
        updateQueueUI();
        try {
            const result = await invoke('analyze_pdf', { path: fileQueue[i].path });
            fileQueue[i].status = 'done';
            fileQueue[i].result = result;
            queueResults.push({ path: fileQueue[i].path, result });
        } catch (err) {
            fileQueue[i].status = 'error';
            fileQueue[i].error = err;
        }
        updateQueueUI();
    }

    const elapsed = Date.now() - queueStartTime;
    if (elapsed < 1200) {
        await new Promise(r => setTimeout(r, 1200 - elapsed));
    }
    showLoading(false);
}

function showQueueView() {
    dropZoneContainer.classList.add('hidden');
    analysisView.classList.add('hidden');
    headerActions.classList.remove('hidden');

    let queueContainer = document.getElementById('queue-view');
    if (!queueContainer) {
        queueContainer = document.createElement('main');
        queueContainer.id = 'queue-view';
        queueContainer.className = 'queue-view';
        document.getElementById('app').insertBefore(queueContainer, document.getElementById('loading-overlay'));
    }
    queueContainer.classList.remove('hidden');
    updateQueueUI();
}

function formatFileSize(bytes) {
    if (!bytes) return '';
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function getQueuePageCount(result) {
    if (!result || !result.findings || result.findings.length === 0) return 0;
    return Math.max(...result.findings.map(f => f.page || 0));
}

function updateQueueUI() {
    const container = document.getElementById('queue-view');
    if (!container) return;

    const doneCount = fileQueue.filter(f => f.status === 'done').length;
    const confirmedSafeCount = fileQueue.filter(f => f.status === 'done' && f.result.verdict === 'Safe' && f.llmResult).length;
    const preliminaryCount = fileQueue.filter(f => f.status === 'done' && f.result.verdict === 'Safe' && !f.llmResult).length;
    const unsafeCount = fileQueue.filter(f => f.status === 'done' && f.result.verdict !== 'Safe').length;
    const pendingCount = fileQueue.filter(f => f.status === 'pending' || f.status === 'processing').length;
    const total = fileQueue.length;
    const progressPct = Math.round((doneCount / total) * 100);
    const isProcessing = fileQueue.some(f => f.status === 'processing');

    // Summary stats
    const statsHtml =
        '<div class="queue-stats">' +
            (confirmedSafeCount > 0 ? '<span class="queue-stat queue-stat-safe' + (queueFilter === 'safe' ? ' active' : '') + '" data-filter="safe">' + ICON.shieldCheck + ' ' + confirmedSafeCount + ' ' + t(confirmedSafeCount === 1 ? 'safe_short' : 'safe_short_plural') + '</span>' : '') +
            (preliminaryCount > 0 ? '<span class="queue-stat queue-stat-preliminary' + (queueFilter === 'preliminary' ? ' active' : '') + '" data-filter="preliminary">' + ICON.shieldCheck + ' ' + preliminaryCount + ' ' + t(preliminaryCount === 1 ? 'preliminary_short' : 'preliminary_short_plural') + '</span>' : '') +
            (unsafeCount > 0 ? '<span class="queue-stat queue-stat-unsafe' + (queueFilter === 'unsafe' ? ' active' : '') + '" data-filter="unsafe">' + ICON.shieldAlert + ' ' + unsafeCount + ' ' + t(unsafeCount === 1 ? 'unsafe_short' : 'unsafe_short_plural') + '</span>' : '') +
            (pendingCount > 0 ? '<span class="queue-stat queue-stat-pending' + (queueFilter === 'pending' ? ' active' : '') + '" data-filter="pending">' + ICON.clock + ' ' + pendingCount + ' ' + t(pendingCount === 1 ? 'pending_short' : 'pending_short_plural') + '</span>' : '') +
        '</div>';

    // Progress
    const progressLabel = doneCount + ' / ' + total + ' ' + t('analyzed_label');
    const progressBarClass = isProcessing ? 'queue-progress-fill queue-progress-animated' : 'queue-progress-fill';

    container.innerHTML =
        '<div class="queue-header">' +
            '<h2>' + t('queue_title') + '</h2>' +
            statsHtml +
            '<div class="queue-progress-section">' +
                '<span class="queue-progress-label">' + progressLabel + '</span>' +
                '<div class="queue-progress-bar"><div class="' + progressBarClass + '" style="width:' + progressPct + '%"></div></div>' +
            '</div>' +
        '</div>' +
        '<div class="queue-list">' +
        fileQueue.map((f, i) => {
            const icon = f.status === 'done' ? (f.result.verdict === 'Safe' ? ICON.shieldCheck : ICON.shieldAlert)
                : f.status === 'processing' ? ICON.loader
                : f.status === 'error' ? ICON.xCircle : ICON.clock;

            const verdictClass = f.status === 'done'
                ? (f.result.verdict === 'Safe' ? (f.llmResult ? 'queue-safe' : 'queue-preliminary') : 'queue-unsafe')
                : f.status === 'processing' ? 'queue-processing' : '';

            const clickable = f.status === 'done';
            const dataAttr = clickable ? ' data-queue-index="' + i + '"' : '';
            // Filter logic
            let matchesFilter = true;
            if (queueFilter) {
                if (queueFilter === 'safe') matchesFilter = f.status === 'done' && f.result.verdict === 'Safe' && f.llmResult;
                else if (queueFilter === 'preliminary') matchesFilter = f.status === 'done' && f.result.verdict === 'Safe' && !f.llmResult;
                else if (queueFilter === 'unsafe') matchesFilter = f.status === 'done' && f.result.verdict !== 'Safe';
                else if (queueFilter === 'pending') matchesFilter = f.status === 'pending' || f.status === 'processing';
            }
            const classes = 'queue-item ' + verdictClass + (clickable ? ' clickable' : '') + ' queue-item-enter' + (!matchesFilter ? ' queue-item-filtered' : '');
            const animDelay = ' style="animation-delay: ' + (i * 0.05) + 's"';

            // Status badge
            let badge = '';
            if (f.status === 'done') {
                if (f.result.verdict === 'Safe') {
                    if (f.llmResult) {
                        badge = '<span class="queue-badge badge-safe">' + t('safe_short') + '</span>';
                    } else {
                        badge = '<span class="queue-badge badge-preliminary">' + t('preliminary_short') + '</span>';
                    }
                } else {
                    const count = f.result.findings.length;
                    badge = '<span class="queue-badge badge-unsafe">' + count + ' ' + t(count === 1 ? 'finding_single' : 'findings_count') + '</span>';
                }
            } else if (f.status === 'processing') {
                badge = '<span class="queue-badge badge-processing">' + t('processing_short') + '</span>';
            } else if (f.status === 'error') {
                badge = '<span class="queue-badge badge-error">' + t('error') + '</span>';
            }

            // LLM badge
            const llmBadge = (f.status === 'done' && f.llmResult)
                ? '<span class="queue-badge badge-llm">' + ICON.bot + ' ' + (f.llmResult.classification === 'injection' ? t('llm_class_injection') : t('llm_class_safe')) + ' ' + f.llmResult.confidence + '%</span>'
                : '';

            // Meta info (file size + pages)
            const size = formatFileSize(f.fileSize);
            const pages = f.status === 'done' ? getQueuePageCount(f.result) : 0;
            let meta = size;
            if (pages > 0) meta += (meta ? ' · ' : '') + pages + ' ' + t('pages_short');
            const metaHtml = meta ? '<span class="queue-item-meta">' + meta + '</span>' : '';

            // Chevron for clickable items
            const chevron = clickable ? '<svg class="queue-chevron" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>' : '';

            return '<div class="' + classes + '"' + dataAttr + animDelay + '>' +
                '<div class="queue-item-left">' +
                    '<div class="queue-item-icon">' + icon + '</div>' +
                    '<div class="queue-item-content">' +
                        '<div class="queue-item-name">' + escapeHtml(f.name) + '</div>' +
                        '<div class="queue-item-details">' + metaHtml + badge + llmBadge + '</div>' +
                    '</div>' +
                '</div>' +
                chevron +
            '</div>';
        }).join('') +
        '</div>';

    // Click handlers to view individual results
    container.querySelectorAll('[data-queue-index]').forEach(el => {
        el.addEventListener('click', () => {
            const idx = parseInt(el.dataset.queueIndex);
            const item = fileQueue[idx];
            currentResult = item.result;
            currentFilePath = item.path;
            currentLlmResult = item.llmResult || null;
            document.getElementById('queue-view').classList.add('hidden');
            backToQueueBtn.classList.remove('hidden');
            showResults();
            if (currentLlmResult) displayLlmResult();
        });
    });

    // Stat filter click handlers
    container.querySelectorAll('[data-filter]').forEach(el => {
        el.style.cursor = 'pointer';
        el.addEventListener('click', () => {
            const filter = el.dataset.filter;
            queueFilter = (queueFilter === filter) ? null : filter;
            updateQueueUI();
        });
    });
}

// Back to queue
backToQueueBtn.addEventListener('click', () => {
    analysisView.classList.add('hidden');
    backToQueueBtn.classList.add('hidden');
    const queueView = document.getElementById('queue-view');
    if (queueView) {
        queueView.classList.remove('hidden');
        // Reset filter if no items match anymore
        if (queueFilter) {
            const hasMatch = fileQueue.some(f => {
                if (queueFilter === 'safe') return f.status === 'done' && f.result.verdict === 'Safe' && f.llmResult;
                if (queueFilter === 'preliminary') return f.status === 'done' && f.result.verdict === 'Safe' && !f.llmResult;
                if (queueFilter === 'unsafe') return f.status === 'done' && f.result.verdict !== 'Safe';
                if (queueFilter === 'pending') return f.status === 'pending' || f.status === 'processing';
                return false;
            });
            if (!hasMatch) queueFilter = null;
        }
        updateQueueUI();
    }
});

async function analyzePdfFile(path) {
    currentFilePath = path;
    document.getElementById('loading-text').textContent = t('analyzing') || 'Analyzing...';
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
        if (currentLlmResult) {
            // LLM confirmed
            if (currentLlmResult.classification === 'injection') {
                verdictBanner.innerHTML = ICON.shieldAlert + ' ' + t('unsafe');
                verdictBanner.className = 'unsafe';
            } else {
                verdictBanner.innerHTML = ICON.shieldCheck + ' ' + t('safe');
                verdictBanner.className = 'safe';
            }
        } else {
            // Heuristic only — preliminary
            verdictBanner.innerHTML = ICON.shieldCheck + ' ' + t('preliminary_safe');
            verdictBanner.className = 'preliminary';
        }
    } else {
        verdictBanner.innerHTML = ICON.shieldAlert + ' ' + t('unsafe');
        verdictBanner.className = 'unsafe';
    }

    // File info bar
    const fileInfoBar = document.getElementById('file-info-bar');
    const fileName = currentResult.filename || currentFilePath.split('/').pop().split('\\').pop();
    const shortHash = currentResult.file_hash ? currentResult.file_hash.slice(0, 12) : '';
    const analyzedAt = currentResult.analyzed_at || '';
    fileInfoBar.innerHTML =
        '<span class="file-info-item">' + escapeHtml(fileName) + '</span>' +
        (shortHash ? '<span class="file-info-separator"></span><span class="file-info-item">SHA256: ' + shortHash + '...</span>' : '') +
        (analyzedAt ? '<span class="file-info-separator"></span><span class="file-info-item">' + analyzedAt + '</span>' : '');
    fileInfoBar.classList.remove('hidden');

    // Findings summary bar
    const summaryBar = document.getElementById('findings-summary-bar');
    const criticalCount = currentResult.findings.filter(f => f.severity === 'Critical').length;
    const warningCount = currentResult.findings.filter(f => f.severity !== 'Critical').length;
    if (currentResult.findings.length > 0) {
        let summaryHtml = '';
        if (criticalCount > 0) summaryHtml += '<span class="summary-item"><span class="summary-dot summary-dot-critical"></span>' + criticalCount + ' Critical</span>';
        if (warningCount > 0) summaryHtml += '<span class="summary-item"><span class="summary-dot summary-dot-warning"></span>' + warningCount + ' Warning</span>';
        summaryBar.innerHTML = summaryHtml;
        summaryBar.classList.remove('hidden');
    } else {
        summaryBar.classList.add('hidden');
    }

    // Findings
    findingsList.innerHTML = '';
    if (currentResult.findings.length === 0) {
        const deepAnalysisCta = !currentLlmResult
            ? '<button class="deep-analysis-cta" id="cta-deep-analysis">' +
                  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="M11 8v6"/><path d="M8 11h6"/></svg> ' +
                  t('run_deep_analysis') +
              '</button>'
            : '';
        const iconColor = currentLlmResult ? 'var(--accent-green)' : '#ffc107';
        findingsList.innerHTML =
            '<div class="safe-empty-state">' +
                '<div class="safe-icon"><svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="color: ' + iconColor + '"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></svg></div>' +
                '<div class="safe-title ' + (currentLlmResult ? 'confirmed' : 'preliminary') + '">' + t(currentLlmResult ? 'no_findings_title_confirmed' : 'no_findings_title') + '</div>' +
                '<div class="safe-subtitle">' + t(currentLlmResult ? 'no_findings_subtitle_confirmed' : 'no_findings_subtitle') + '</div>' +
                deepAnalysisCta +
            '</div>';
        // CTA click handler
        const ctaBtn = document.getElementById('cta-deep-analysis');
        if (ctaBtn) {
            ctaBtn.addEventListener('click', () => deepAnalysisBtn.click());
        }
    } else {
        // Group findings by page
        const grouped = {};
        currentResult.findings.forEach((finding, idx) => {
            const page = finding.page || 0;
            if (!grouped[page]) grouped[page] = [];
            grouped[page].push({ finding, idx });
        });

        const sortedPages = Object.keys(grouped).map(Number).sort((a, b) => a - b);
        let animIdx = 0;

        sortedPages.forEach(page => {
            const items = grouped[page];
            const pageLabel = page === 0 ? t('general_section') : t('page') + ' ' + page;
            const countLabel = items.length + ' ' + (items.length === 1 ? t('finding_single') : t('findings_count'));

            const group = document.createElement('div');
            group.className = 'findings-page-group';
            group.style.animationDelay = (animIdx * 0.08) + 's';

            const header = document.createElement('div');
            header.className = 'findings-page-header';
            if (page > 0) header.dataset.page = page;
            header.innerHTML =
                '<div class="findings-page-title">' +
                    '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>' +
                    ' ' + pageLabel +
                '</div>' +
                '<span class="findings-page-count">' + countLabel + '</span>';
            group.appendChild(header);

            const list = document.createElement('div');
            list.className = 'findings-page-items';

            items.forEach(({ finding, idx: findingIdx }) => {
                const severity = finding.severity === 'Critical' ? 'critical' : 'warning';
                const icon = finding.severity === 'Critical' ? ICON.circleAlert : ICON.triangleAlert;
                const typeTag = t('detection_type_' + finding.detection_type) || finding.detection_type;
                const description = t('desc_' + finding.detection_type) || finding.description;
                const infoDesc = t('info_' + finding.detection_type);
                const el = document.createElement('div');
                el.className = 'finding-item ' + severity;
                el.style.animationDelay = (animIdx * 0.06) + 's';
                animIdx++;
                const excerptText = finding.char_offset == null
                    ? (tOptional('excerpt_' + finding.detection_type) || finding.excerpt)
                    : finding.excerpt;
                el.innerHTML =
                    '<div class="finding-header">' +
                        '<span class="finding-tag tag-' + severity + '" data-info-type="' + finding.detection_type + '">' + escapeHtml(typeTag) + '</span> ' +
                        icon + ' ' + escapeHtml(description) +
                    '</div>' +
                    (finding.excerpt ? '<div class="finding-excerpt">' + escapeHtml(excerptText) + '</div>' : '') +
                    '<button class="finding-expand-btn" data-finding-idx="' + findingIdx + '">' +
                        '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>' +
                        ' ' + t('details_label') +
                    '</button>' +
                    '<div class="finding-info">' + escapeHtml(infoDesc) + '</div>';
                list.appendChild(el);
            });

            group.appendChild(list);
            findingsList.appendChild(group);
        });

        // Expand/collapse handlers
        findingsList.querySelectorAll('.finding-expand-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const info = btn.nextElementSibling;
                info.classList.toggle('visible');
                btn.classList.toggle('expanded');
            });
        });

        // Click page header to navigate PDF
        findingsList.querySelectorAll('.findings-page-header[data-page]').forEach(header => {
            header.addEventListener('click', async () => {
                const page = parseInt(header.dataset.page);
                if (page > 0 && page <= getPageCount()) {
                    currentPdfPage = page;
                    await goToPage(page, currentResult ? currentResult.findings : []);
                    refreshPdfNav();
                }
            });
        });
    }

    // PDF page nav
    updatePdfNav();
    renderPdf(currentFilePath);
}

function updatePdfNav() {
    currentPdfPage = 1;
    const total = getPageCount();
    const indicator = document.getElementById('pdf-page-indicator');
    const prevBtn = document.getElementById('pdf-prev-btn');
    const nextBtn = document.getElementById('pdf-next-btn');
    if (indicator) indicator.textContent = '1 / ' + (total || 1);
    if (prevBtn) prevBtn.disabled = true;
    if (nextBtn) nextBtn.disabled = total <= 1;
}

async function renderPdf(path) {
    try {
        const { readFile } = window.__TAURI__.fs;
        const fileBytes = await readFile(path);
        await renderPdfFromBytes(fileBytes, currentResult ? currentResult.findings : []);
        updatePdfNav();
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

// PDF page navigation
document.getElementById('pdf-prev-btn').addEventListener('click', async () => {
    if (currentPdfPage > 1) {
        currentPdfPage--;
        await goToPage(currentPdfPage, currentResult ? currentResult.findings : []);
        refreshPdfNav();
    }
});
document.getElementById('pdf-next-btn').addEventListener('click', async () => {
    const total = getPageCount();
    if (currentPdfPage < total) {
        currentPdfPage++;
        await goToPage(currentPdfPage, currentResult ? currentResult.findings : []);
        refreshPdfNav();
    }
});

function refreshPdfNav() {
    const total = getPageCount();
    const indicator = document.getElementById('pdf-page-indicator');
    const prevBtn = document.getElementById('pdf-prev-btn');
    const nextBtn = document.getElementById('pdf-next-btn');
    if (indicator) indicator.textContent = currentPdfPage + ' / ' + (total || 1);
    if (prevBtn) prevBtn.disabled = currentPdfPage <= 1;
    if (nextBtn) nextBtn.disabled = currentPdfPage >= total;
}

// Deep Analysis
deepAnalysisBtn.addEventListener('click', async () => {
    const queueView = document.getElementById('queue-view');
    const isQueueVisible = queueView && !queueView.classList.contains('hidden');

    if (isQueueVisible && queueResults.length > 0) {
        // Batch deep analysis for all queue items
        showLoading(true);
        try {
            for (let i = 0; i < fileQueue.length; i++) {
                if (fileQueue[i].status !== 'done') continue;
                document.getElementById('loading-text').textContent =
                    (t('deep_analysis') || 'Deep Analysis') + ' (' + (i + 1) + '/' + fileQueue.length + ')';
                const result = fileQueue[i].result;
                let textForLlm = result.extracted_text || '';
                if (result.findings.length > 0) {
                    textForLlm += '\n\n--- HEURISTIC FINDINGS (hidden/suspicious content detected) ---\n';
                    result.findings.forEach((f, j) => {
                        textForLlm += `${j+1}. [${f.detection_type}] Page ${f.page}: ${f.description}\n   Excerpt: "${f.excerpt}"\n`;
                    });
                }
                const llmResult = await invoke('deep_analysis', { text: textForLlm || 'No text extracted from PDF' });
                fileQueue[i].llmResult = llmResult;
            }
            updateQueueUI();
        } catch (err) {
            alert(t('llm_analysis_header') + ' ' + t('error').toLowerCase() + ': ' + err);
        } finally {
            showLoading(false);
        }
        return;
    }

    if (!currentResult) return;

    showLoading(true);
    document.getElementById('loading-text').textContent = t('deep_analysis') + '...';
    try {
        let textForLlm = currentResult.extracted_text || '';
        if (currentResult.findings.length > 0) {
            textForLlm += '\n\n--- HEURISTIC FINDINGS (hidden/suspicious content detected) ---\n';
            currentResult.findings.forEach((f, i) => {
                textForLlm += `${i+1}. [${f.detection_type}] Page ${f.page}: ${f.description}\n   Excerpt: "${f.excerpt}"\n`;
            });
        }
        currentLlmResult = await invoke('deep_analysis', { text: textForLlm || 'No text extracted from PDF' });
        // Persist LLM result back to queue item if applicable
        const queueIdx = fileQueue.findIndex(f => f.path === currentFilePath);
        if (queueIdx >= 0) fileQueue[queueIdx].llmResult = currentLlmResult;
        displayLlmResult();
    } catch (err) {
        alert(t('llm_analysis_header') + ' ' + t('error').toLowerCase() + ': ' + err);
    } finally {
        showLoading(false);
    }
});

function displayLlmResult() {
    if (!currentLlmResult) return;
    const existing = document.getElementById('llm-result');
    if (existing) existing.remove();

    // Update verdict banner now that LLM is available
    if (currentResult && currentResult.verdict === 'Safe') {
        if (currentLlmResult.classification === 'injection') {
            verdictBanner.innerHTML = ICON.shieldAlert + ' ' + t('unsafe');
            verdictBanner.className = 'unsafe';
        } else {
            verdictBanner.innerHTML = ICON.shieldCheck + ' ' + t('safe');
            verdictBanner.className = 'safe';
        }
    }

    // Remove CTA button if present
    const cta = document.getElementById('cta-deep-analysis');
    if (cta) cta.remove();

    // Update safe empty state title, subtitle, and icon after LLM confirmation
    const safeTitle = findingsList.querySelector('.safe-title');
    if (safeTitle && currentLlmResult.classification !== 'injection') {
        safeTitle.textContent = t('no_findings_title_confirmed');
        safeTitle.className = 'safe-title confirmed';
    }
    const safeSubtitle = findingsList.querySelector('.safe-subtitle');
    if (safeSubtitle && currentLlmResult.classification !== 'injection') {
        safeSubtitle.textContent = t('no_findings_subtitle_confirmed');
    }
    const safeIcon = findingsList.querySelector('.safe-icon svg');
    if (safeIcon && currentLlmResult.classification !== 'injection') {
        safeIcon.style.color = 'var(--accent-green)';
    }

    const isInjection = currentLlmResult.classification === 'injection';
    const classLabel = isInjection ? t('llm_class_injection') : t('llm_class_safe');
    const borderColor = isInjection ? 'var(--accent)' : 'var(--accent-green)';
    const badgeClass = isInjection ? 'badge-unsafe' : 'badge-safe';

    const el = document.createElement('div');
    el.id = 'llm-result';
    el.className = 'llm-result-section';
    el.innerHTML =
        '<div class="llm-result-header">' +
            '<div class="llm-result-title">' + ICON.bot + ' ' + t('llm_analysis_header') + '</div>' +
            '<span class="queue-badge ' + badgeClass + '">' + escapeHtml(classLabel) + ' · ' + currentLlmResult.confidence + '%</span>' +
        '</div>' +
        '<div class="llm-result-body">' +
            '<div class="llm-result-explanation">' + escapeHtml(currentLlmResult.explanation) + '</div>' +
        '</div>';
    el.style.borderColor = borderColor;
    findingsList.appendChild(el);
}

// Export
exportBtn.addEventListener('click', async () => {
    // If viewing a single result, export it
    if (currentResult) {
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
        return;
    }

    // Queue mode: export single consolidated report
    const completed = fileQueue.filter(f => f.status === 'done');
    if (completed.length === 0) return;

    const outputPath = await save({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        defaultPath: 'batch-report.pdf',
    });
    if (outputPath) {
        try {
            await invoke('export_batch_report', {
                results: completed.map(f => f.result),
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
    fileQueue = [];
    queueResults = [];
    analysisView.classList.add('hidden');
    headerActions.classList.add('hidden');
    backToQueueBtn.classList.add('hidden');
    const queueView = document.getElementById('queue-view');
    if (queueView) queueView.classList.add('hidden');
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

// Display version
getVersion().then(v => {
    document.getElementById('app-version').textContent = 'v' + v;
});

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
            <div class="info-item ${severity}" data-type="${type}">
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

// Click on finding tag → toggle inline info description
findingsList.addEventListener('click', (e) => {
    const tag = e.target.closest('[data-info-type]');
    if (!tag) return;
    const item = tag.closest('.finding-item');
    if (!item) return;
    const info = item.querySelector('.finding-info');
    const btn = item.querySelector('.finding-expand-btn');
    if (info) {
        info.classList.toggle('visible');
        if (btn) btn.classList.toggle('expanded');
    }
});
