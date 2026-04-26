/* Boltr Bldr — WebUI */

const API = '';

document.querySelectorAll('.nav-links a').forEach(link => {
    link.addEventListener('click', e => {
        e.preventDefault();
        const page = link.dataset.page;
        document.querySelectorAll('.nav-links a').forEach(l => l.classList.remove('active'));
        document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
        link.classList.add('active');
        document.getElementById(`page-${page}`).classList.add('active');
    });
});

async function apiGet(path) {
    const resp = await fetch(`${API}${path}`);
    return parseApiResponse(resp);
}

async function apiPost(path, body = {}) {
    const resp = await fetch(`${API}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });
    return parseApiResponse(resp);
}

async function parseApiResponse(resp) {
    const text = await resp.text();
    let payload = null;
    if (text) {
        try {
            payload = JSON.parse(text);
        } catch {
            payload = { success: false, error: text };
        }
    }

    if (!resp.ok) {
        const message = payload && payload.error ? payload.error : `${resp.status} ${resp.statusText}`;
        throw new Error(message);
    }

    return payload || { success: true, data: null };
}

function showResult(elId, data, isError = false) {
    const el = document.getElementById(elId);
    el.classList.remove('hidden', 'success', 'error');
    el.classList.add(isError ? 'error' : 'success');
    if (typeof data === 'object') {
        el.textContent = JSON.stringify(data, null, 2);
    } else {
        el.textContent = data;
    }
}

function parseIds(str) {
    return str.trim().split(/[\s,]+/).filter(s => s.length > 0);
}

function parseSeeds(str) {
    const parts = str.split(/[\s,]+/).filter(s => s.length > 0);
    if (parts.length === 0) return [1];
    return parts.map(s => parseInt(s, 10)).filter(n => !Number.isNaN(n));
}

// ── Dashboard ───────────────────────────────────────────────────────

async function loadDashboard() {
    try {
        const resp = await apiGet('/api/status');
        if (resp.success && resp.data) {
            const stats = resp.data.stats || {};
            document.getElementById('stat-artifacts').textContent = stats.total_artifacts ?? '0';
            document.getElementById('stat-yaml').textContent = stats.total_yaml ?? '0';
            document.getElementById('stat-npz').textContent = stats.total_npz ?? '0';
            document.getElementById('stat-packages').textContent = stats.total_packages ?? '0';
            const sizeMB = ((stats.total_size_bytes || 0) / 1048576).toFixed(2);
            document.getElementById('stat-size').textContent = `${sizeMB} MB`;

            document.getElementById('status-bar').textContent = `ok - data_dir: ${resp.data.data_dir || 'data'}`;
        }
    } catch (e) {
        document.getElementById('status-bar').textContent = `error: ${e.message}`;
    }
}

// ── Job builder (AF3-style) ───────────────────────────────────────────

let lastYamlText = '';

function entityFieldsHtml(kind) {
    if (kind === 'ligand') {
        return `
            <label class="entity-path-hint">SMILES (optional if CCD set)</label>
            <input type="text" class="entity-smiles" placeholder="e.g. CC(=O)O" />
            <label class="entity-path-hint">CCD codes (comma-separated)</label>
            <input type="text" class="entity-ccd" placeholder="ATP, MG" />
        `;
    }
    const ph =
        kind === 'protein'
            ? '>FASTA or one-letter sequence'
            : kind === 'dna'
              ? 'DNA: A C G T'
              : 'RNA: A C G U';
    return `
        <textarea class="entity-seq" placeholder="${ph}"></textarea>
        <div class="field-row">
            <input type="file" class="entity-file" accept=".cif,.mmcif,.pdb,.ent" />
            <span class="entity-path-hint entity-upload-status"></span>
        </div>
    `;
}

function createEntityRow(kind = 'protein') {
    const wrap = document.createElement('div');
    wrap.className = 'entity-row';
    wrap.setAttribute('draggable', 'true');
    wrap.innerHTML = `
        <div class="entity-head">
            <span class="entity-drag" title="Drag to reorder">⠿</span>
            <select class="entity-kind">
                <option value="protein" ${kind === 'protein' ? 'selected' : ''}>protein</option>
                <option value="dna" ${kind === 'dna' ? 'selected' : ''}>dna</option>
                <option value="rna" ${kind === 'rna' ? 'selected' : ''}>rna</option>
                <option value="ligand" ${kind === 'ligand' ? 'selected' : ''}>ligand</option>
            </select>
            <span class="entity-copies-wrap">copies <input type="number" class="entity-copies" min="1" value="1" /></span>
            <button type="button" class="btn btn-secondary entity-remove" title="Remove">×</button>
        </div>
        <div class="entity-fields">${entityFieldsHtml(kind)}</div>
    `;

    const kindSel = wrap.querySelector('.entity-kind');
    kindSel.addEventListener('change', () => {
        delete wrap.dataset.mmcifPath;
        delete wrap.dataset.pdbPath;
        wrap.querySelector('.entity-fields').innerHTML = entityFieldsHtml(kindSel.value);
        wireEntityFileUpload(wrap);
    });

    wrap.querySelector('.entity-remove').addEventListener('click', () => {
        const parent = document.getElementById('entity-rows');
        if (parent.children.length <= 1) return;
        wrap.remove();
    });

    wireEntityFileUpload(wrap);
    wireDragRow(wrap);
    return wrap;
}

function wireEntityFileUpload(row) {
    const fileInput = row.querySelector('.entity-file');
    if (!fileInput) return;
    const status = row.querySelector('.entity-upload-status');
    fileInput.addEventListener('change', async () => {
        const f = fileInput.files && fileInput.files[0];
        if (!f) return;
        status.textContent = 'uploading…';
        const fd = new FormData();
        fd.append('file', f);
        try {
            const resp = await fetch(`${API}/api/upload-structure`, { method: 'POST', body: fd });
            const json = await parseApiResponse(resp);
            if (json.success && json.data && json.data.path) {
                const p = json.data.path;
                const lower = f.name.toLowerCase();
                row.dataset.mmcifPath = '';
                row.dataset.pdbPath = '';
                if (lower.endsWith('.pdb') || lower.endsWith('.ent')) {
                    row.dataset.pdbPath = p;
                } else {
                    row.dataset.mmcifPath = p;
                }
                status.textContent = `template: ${p}`;
            } else {
                status.textContent = json.error || 'upload failed';
            }
        } catch (e) {
            status.textContent = e.message;
        }
    });
}

function wireDragRow(row) {
    row.addEventListener('dragstart', e => {
        row.classList.add('dragging');
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', '');
    });
    row.addEventListener('dragend', () => row.classList.remove('dragging'));
}

function initEntityDragDrop(container) {
    container.addEventListener('dragover', e => {
        e.preventDefault();
        const dragging = container.querySelector('.dragging');
        if (!dragging) return;
        const after = [...container.querySelectorAll('.entity-row:not(.dragging)')].find(el => {
            const r = el.getBoundingClientRect();
            return e.clientY < r.top + r.height / 2;
        });
        if (after) {
            container.insertBefore(dragging, after);
        } else {
            container.appendChild(dragging);
        }
    });
}

function collectEntities() {
    const rows = [...document.querySelectorAll('#entity-rows .entity-row')];
    return rows.map(row => {
        const kind = row.querySelector('.entity-kind').value;
        const copies = Math.max(1, parseInt(row.querySelector('.entity-copies').value, 10) || 1);
        const base = {
            kind,
            copies,
            sequence: '',
            smiles: '',
            ccd_codes: [],
            mmcif_path: null,
            pdb_path: null,
            description: null,
        };
        if (kind === 'ligand') {
            const smiles = row.querySelector('.entity-smiles');
            const ccd = row.querySelector('.entity-ccd');
            base.smiles = smiles ? smiles.value.trim() : '';
            const ccdStr = ccd ? ccd.value.trim() : '';
            base.ccd_codes = ccdStr ? ccdStr.split(/[\s,]+/).filter(Boolean) : [];
        } else {
            const ta = row.querySelector('.entity-seq');
            base.sequence = ta ? ta.value : '';
            if (row.dataset.mmcifPath) base.mmcif_path = row.dataset.mmcifPath;
            if (row.dataset.pdbPath) base.pdb_path = row.dataset.pdbPath;
        }
        return base;
    });
}

document.getElementById('btn-add-entity').addEventListener('click', () => {
    document.getElementById('entity-rows').appendChild(createEntityRow('protein'));
});

document.getElementById('btn-generate-yaml').addEventListener('click', async () => {
    const name = document.getElementById('job-name').value.trim() || 'manual-job';
    const seeds = parseSeeds(document.getElementById('job-seeds').value);
    const version = document.getElementById('schema-version').value.trim() || '1.0.0';
    const entities = collectEntities();

    const btn = document.getElementById('btn-generate-yaml');
    btn.disabled = true;
    btn.textContent = 'generating…';

    try {
        const resp = await apiPost('/api/job-yaml', {
            name,
            model_seeds: seeds,
            version,
            entities,
        });
        if (resp.success && resp.data && resp.data.yaml) {
            lastYamlText = resp.data.yaml;
            showResult('job-yaml-result', resp.data.yaml, false);
            document.getElementById('btn-download-yaml').disabled = false;
        } else {
            lastYamlText = '';
            showResult('job-yaml-result', resp, true);
            document.getElementById('btn-download-yaml').disabled = true;
        }
    } catch (e) {
        lastYamlText = '';
        showResult('job-yaml-result', `network error: ${e.message}`, true);
        document.getElementById('btn-download-yaml').disabled = true;
    } finally {
        btn.disabled = false;
        btn.textContent = 'Generate YAML';
    }
});

document.getElementById('btn-download-yaml').addEventListener('click', () => {
    if (!lastYamlText) return;
    const blob = new Blob([lastYamlText], { type: 'text/yaml;charset=utf-8' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${(document.getElementById('job-name').value || 'job').trim() || 'job'}.boltr.yaml`;
    a.click();
    URL.revokeObjectURL(a.href);
});

document.getElementById('btn-clear-job').addEventListener('click', () => {
    document.getElementById('job-name').value = 'manual-job';
    document.getElementById('job-seeds').value = '1';
    document.getElementById('schema-version').value = '1.0.0';
    const er = document.getElementById('entity-rows');
    er.innerHTML = '';
    er.appendChild(createEntityRow('protein'));
    lastYamlText = '';
    document.getElementById('job-yaml-result').classList.add('hidden');
    document.getElementById('btn-download-yaml').disabled = true;
});

// ── Legacy ingest ─────────────────────────────────────────────────────

document.getElementById('btn-ingest').addEventListener('click', async () => {
    const pdbIds = parseIds(document.getElementById('ingest-pdb').value);
    const uniprotIds = parseIds(document.getElementById('ingest-uniprot').value);

    if (pdbIds.length === 0 && uniprotIds.length === 0) {
        showResult('ingest-result', 'Error: enter at least one PDB ID or UniProt accession', true);
        return;
    }

    const btn = document.getElementById('btn-ingest');
    btn.disabled = true;
    btn.textContent = 'Fetching…';

    try {
        const resp = await apiPost('/api/ingest', { pdb: pdbIds, uniprot: uniprotIds });
        showResult('ingest-result', resp);
    } catch (e) {
        showResult('ingest-result', `Network error: ${e.message}`, true);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Fetch data';
    }
});

document.getElementById('btn-normalize').addEventListener('click', async () => {
    const btn = document.getElementById('btn-normalize');
    btn.disabled = true;
    btn.textContent = 'Normalizing…';

    try {
        const resp = await apiPost('/api/normalize', {});
        showResult('normalize-result', resp);
    } catch (e) {
        showResult('normalize-result', `Error: ${e.message}`, true);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Normalize';
    }
});

document.getElementById('btn-emit').addEventListener('click', async () => {
    const version = document.getElementById('emit-version').value || '1.0.0';
    const btn = document.getElementById('btn-emit');
    btn.disabled = true;
    btn.textContent = 'Emitting…';

    try {
        const resp = await apiPost('/api/emit', { version });
        showResult('emit-result', resp);
    } catch (e) {
        showResult('emit-result', `Error: ${e.message}`, true);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Emit YAML';
    }
});

// ── Artifacts ─────────────────────────────────────────────────────────

async function loadArtifacts() {
    try {
        const resp = await apiGet('/api/artifacts?limit=100&offset=0');
        const tbody = document.getElementById('artifacts-body');

        if (resp.success && resp.data && resp.data.artifacts && resp.data.artifacts.length > 0) {
            tbody.innerHTML = resp.data.artifacts.map(a => `
                <tr>
                    <td>${esc(a.file_type)}</td>
                    <td class="mono">${esc(a.file_path)}</td>
                    <td>${esc(a.source_db)}:${esc(a.source_id)}</td>
                    <td>${formatBytes(a.size_bytes)}</td>
                    <td class="mono">${esc(a.sha256 ? a.sha256.substring(0, 16) + '…' : '—')}</td>
                </tr>
            `).join('');
        } else {
            tbody.innerHTML = '<tr><td colspan="5" class="empty">No artifacts indexed</td></tr>';
        }
    } catch (e) {
        console.error('Failed to load artifacts:', e);
    }
}

async function loadPackages() {
    try {
        const resp = await apiGet('/api/packages');
        const tbody = document.getElementById('packages-body');

        if (resp.success && resp.data && resp.data.packages && resp.data.packages.length > 0) {
            tbody.innerHTML = resp.data.packages.map(p => `
                <tr>
                    <td class="mono">${esc(p.package_id)}</td>
                    <td>${p.file_count}</td>
                    <td>${formatBytes(p.total_size)}</td>
                    <td>${esc(formatDate(p.created_at))}</td>
                    <td>${esc((p.tags || []).join(', ') || '-')}</td>
                </tr>
            `).join('');
        } else {
            tbody.innerHTML = '<tr><td colspan="5" class="empty">No packages found</td></tr>';
        }
    } catch (e) {
        const tbody = document.getElementById('packages-body');
        tbody.innerHTML = `<tr><td colspan="5" class="empty">Failed to load packages: ${esc(e.message)}</td></tr>`;
    }
}

document.getElementById('btn-refresh-artifacts').addEventListener('click', async () => {
    await loadArtifacts();
    await loadDashboard();
});

document.getElementById('btn-refresh-packages').addEventListener('click', async () => {
    await loadPackages();
    await loadDashboard();
});

document.getElementById('btn-reindex').addEventListener('click', async () => {
    const btn = document.getElementById('btn-reindex');
    btn.disabled = true;
    btn.textContent = 'Indexing…';

    try {
        const resp = await apiPost('/api/index', {});
        showResult('artifacts-reindex-result', resp);
        await loadArtifacts();
        await loadPackages();
        await loadDashboard();
    } catch (e) {
        console.error(e);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Re-index all';
    }
});

// ── Utilities ─────────────────────────────────────────────────────────

function esc(str) {
    return String(str || '').replace(/[&<>"']/g, ch => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
    }[ch]));
}

function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function formatDate(value) {
    if (!value) return '-';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleString();
}

// ── Init ──────────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', () => {
    const er = document.getElementById('entity-rows');
    er.appendChild(createEntityRow('protein'));
    initEntityDragDrop(er);
    loadDashboard();
    loadArtifacts();
    loadPackages();
});
