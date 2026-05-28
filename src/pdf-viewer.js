// PDF.js viewer with highlight support
const PDFJS_CDN = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/4.0.379/pdf.min.mjs';
const PDFJS_WORKER = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/4.0.379/pdf.worker.min.mjs';

let pdfDoc = null;
let currentPage = 1;

export async function renderPdfFromBytes(fileBytes, findings) {
    const canvas = document.getElementById('pdf-canvas');
    const ctx = canvas.getContext('2d');

    try {
        const pdfjsLib = await import(PDFJS_CDN);
        pdfjsLib.GlobalWorkerOptions.workerSrc = PDFJS_WORKER;

        pdfDoc = await pdfjsLib.getDocument({ data: fileBytes }).promise;
        await renderPage(1, canvas, findings);
    } catch (err) {
        console.error('PDF render error:', err);
        canvas.width = 500;
        canvas.height = 700;
        ctx.fillStyle = '#1a1a2e';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#e94560';
        ctx.font = '14px sans-serif';
        ctx.fillText('Failed to render PDF: ' + err.message, 20, 40);
    }
}

async function renderPage(pageNum, canvas, findings) {
    if (!pdfDoc) return;

    const page = await pdfDoc.getPage(pageNum);
    const scale = 1.5;
    const viewport = page.getViewport({ scale });

    canvas.width = viewport.width;
    canvas.height = viewport.height;

    const ctx = canvas.getContext('2d');
    await page.render({ canvasContext: ctx, viewport }).promise;

    const pageFindings = findings.filter(f => f.page === pageNum);
    if (pageFindings.length > 0) {
        ctx.strokeStyle = 'rgba(233, 69, 96, 0.8)';
        ctx.lineWidth = 3;
        ctx.strokeRect(2, 2, canvas.width - 4, canvas.height - 4);

        let yOffset = 20;
        for (const finding of pageFindings) {
            const icon = finding.severity === 'Critical' ? '🔴' : '🟡';
            const label = icon + ' ' + (finding.description || '').slice(0, 35);
            ctx.font = '11px sans-serif';
            const textWidth = ctx.measureText(label).width;
            ctx.fillStyle = 'rgba(0, 0, 0, 0.75)';
            ctx.fillRect(canvas.width - textWidth - 20, yOffset - 12, textWidth + 14, 18);
            ctx.fillStyle = '#fff';
            ctx.fillText(label, canvas.width - textWidth - 13, yOffset);
            yOffset += 22;
        }
    }

    currentPage = pageNum;
}

export function getPageCount() {
    return pdfDoc ? pdfDoc.numPages : 0;
}

export async function goToPage(pageNum, findings) {
    const canvas = document.getElementById('pdf-canvas');
    if (pageNum >= 1 && pageNum <= getPageCount()) {
        await renderPage(pageNum, canvas, findings);
    }
}
