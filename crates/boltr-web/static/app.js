/* Boltr Bldr — Frontend Application */

const API = '';  // Same-origin; empty string = relative

// ── Navigation ──────────────────────────────────────────────────────

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

// ── API Helpers ─────────────────────────────────────────────────────

async function apiGet(path) {
    const resp = await fetch(`${API}${path}`);
    return resp.json();
}

async function apiPost(path, body = {}) {
    const resp = await fetch(`${API}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });
    return resp.json();
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

            document.getElementById('status-bar').innerHTML =
                `<span class="status-dot"></span> Running — Data dir: ${resp.data.data_dir || 'data'}`;
        }
    } catch (e) {
        document.getElementById('status-bar').innerHTML =
            `<span class="status-dot" style="background:var(--error)"></span> Error: ${e.message}`;
    }
}

// ── Ingest ───────────────────────────────────────────────────────────

document.getElementById('btn-ingest').addEventListener('click', async () => {
    const pdbIds = parseIds(document.getElementById('ingest-pdb').value);
    const uniprotIds = parseIds(document.getElementById('ingest-uniprot').value);

    if (pdbIds.length === 0 && uniprotIds.length === 0) {
        showResult('ingest-result', 'Error: Enter at least one PDB ID or UniProt accession', true);
        return;
    }

    const btn = document.getElementById('btn-ingest');
    btn.disabled = true;
    btn.textContent = 'Fetching...';

    try {
        const resp = await apiPost('/api/ingest', { pdb: pdbIds, uniprot: uniprotIds });
        showResult('ingest-result', resp);
    } catch (e) {
        showResult('ingest-result', `Network error: ${e.message}`, true);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Fetch Data';
    }
});

// ── Normalize ────────────────────────────────────────────────────────

document.getElementById('btn-normalize').addEventListener('click', async () => {
    const btn = document.getElementById('btn-normalize');
    btn.disabled = true;
    btn.textContent = 'Normalizing...';

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

// ── Emit ────────────────────��────────────────────────────────────────

document.getElementById('btn-emit').addEventListener('click', async () => {
    const version = document.getElementById('emit-version').value || '1.0.0';
    const btn = document.getElementById('btn-emit');
    btn.disabled = true;
    btn.textContent = 'Emitting...';

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

// ── Pipeline ─────────────────────────────────────────────────────────

document.getElementById('btn-pipeline').addEventListener('click', async () => {
    const pdbIds = parseIds(document.getElementById('pipeline-pdb').value);
    const uniprotIds = parseIds(document.getElementById('pipeline-uniprot').value);

    if (pdbIds.length === 0 && uniprotIds.length === 0) {
        showResult('pipeline-result', 'Error: Enter at least one PDB ID or UniProt accession', true);
        return;
    }

    const btn = document.getElementById('btn-pipeline');
    btn.disabled = true;
    btn.textContent = '⏳ Running...';

    try {
        const resp = await apiPost('/api/pipeline', { pdb: pdbIds, uniprot: uniprotIds });
        showResult('pipeline-result', resp);
    } catch (e) {
        showResult('pipeline-result', `Error: ${e.message}`, true);
    } finally {
        btn.disabled = false;
        btn.textContent = '🚀 Run Pipeline';
    }
});

// ── Artifacts ────────────────────────────────────────────────────────

async function loadArtifacts() {
    try {
        const resp = await apiGet('/api/artifacts');
        const tbody = document.getElementById('artifacts-body');

        if (resp.success && resp.data && resp.data.artifacts && resp.data.artifacts.length > 0) {
            tbody.innerHTML = resp.data.artifacts.map(a => `
                <tr>
                    <td>${esc(a.file_type)}</td>
                    <td class="mono">${esc(a.file_path)}</td>
                    <td>${esc(a.source_db)}:${esc(a.source_id)}</td>
                    <td>${formatBytes(a.size_bytes)}</td>
                    <td class="mono">${esc(a.sha256 ? a.sha256.substring(0, 16) + '...' : '—')}</td>
                </tr>
            `).join('');
        } else {
            tbody.innerHTML = '<tr><td colspan="5" class="empty">No artifacts indexed</td></tr>';
        }
    } catch (e) {
        console.error('Failed to load artifacts:', e);
    }
}

document.getElementById('btn-refresh-artifacts').addEventListener('click', loadArtifacts);

document.getElementById('btn-reindex').addEventListener('click', async () => {
    const btn = document.getElementById('btn-reindex');
    btn.disabled = true;
    btn.textContent = 'Indexing...';

    try {
        const resp = await apiPost('/api/index');
        showResult('artifacts-table', resp);  // Brief flash
        await loadArtifacts();
    } catch (e) {
        console.error(e);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Re-index All';
    }
});

// ── Packages ─────────────────────────────────────────────────────────

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
                    <td>${esc(new Date(p.created_at).toLocaleString())}</td>
                    <td>${esc(p.description || '—')}</td>
                </tr>
            `).join('');
        } else {
            tbody.innerHTML = '<tr><td colspan="5" class="empty">No packages found</td></tr>';
        }
    } catch (e) {
        console.error('Failed to load packages:', e);
    }
}

document.getElementById('btn-refresh-packages').addEventListener('click', loadPackages);

document.getElementById('btn-package').addEventListener('click', async () => {
    const btn = document.getElementById('btn-package');
    btn.disabled = true;
    btn.textContent = 'Packaging...';

    try {
        const resp = await apiPost('/api/package', {});
        await loadPackages();
        alert(resp.success ? `Package created: ${resp.data?.package_id}` : `Error: ${resp.error}`);
    } catch (e) {
        alert(`Error: ${e.message}`);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Package Current';
    }
});

// ── Utilities ────────────────────────────────────────────────────────

function esc(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}

function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// ── Initialize ───────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', () => {
    loadDashboard();
    loadArtifacts();
    loadPackages();
});
