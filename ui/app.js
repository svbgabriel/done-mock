// Global State
let mocksCache = [];
let editingMockId = null;
let conditionsState = [];
let sequenceState = [];
let liveFeedEvents = [];
let eventSource = null;

document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    loadMocks();
    initLiveFeedSSE();

    // Global Actions
    document.getElementById('btn-refresh').addEventListener('click', loadMocks);
    document.getElementById('btn-new-mock').addEventListener('click', () => {
        switchTab('mocks');
        resetForm();
        document.getElementById('mock-name').focus();
    });
    document.getElementById('btn-clear-feed').addEventListener('click', clearFeed);

    // Form Events
    document.getElementById('mock-type').addEventListener('change', onMockTypeChange);
    document.getElementById('btn-add-condition').addEventListener('click', addConditionItem);
    document.getElementById('btn-add-sequence-item').addEventListener('click', addSequenceItem);
    document.getElementById('btn-reset-form').addEventListener('click', resetForm);
    document.getElementById('btn-cancel-edit').addEventListener('click', resetForm);
    document.getElementById('mock-form').addEventListener('submit', handleFormSubmit);

    // Initial renders
    renderConditions();
    renderSequenceItems();
});

// Tab Navigation
function initTabs() {
    const tabMocksBtn = document.getElementById('tab-btn-mocks');
    const tabLiveFeedBtn = document.getElementById('tab-btn-livefeed');

    tabMocksBtn.addEventListener('click', () => switchTab('mocks'));
    tabLiveFeedBtn.addEventListener('click', () => switchTab('livefeed'));
}

function switchTab(tabName) {
    const tabMocksBtn = document.getElementById('tab-btn-mocks');
    const tabLiveFeedBtn = document.getElementById('tab-btn-livefeed');
    const tabMocks = document.getElementById('tab-mocks');
    const tabLiveFeed = document.getElementById('tab-livefeed');

    if (tabName === 'mocks') {
        tabMocksBtn.classList.add('active');
        tabLiveFeedBtn.classList.remove('active');
        tabMocks.style.display = 'flex';
        tabLiveFeed.style.display = 'none';
    } else {
        tabMocksBtn.classList.remove('active');
        tabLiveFeedBtn.classList.add('active');
        tabMocks.style.display = 'none';
        tabLiveFeed.style.display = 'flex';
    }
}

// Alerts
function showAlert(message, type = 'success', durationMs = 4000) {
    const container = document.getElementById('alert-container');
    const alert = document.createElement('div');
    alert.className = `alert alert-${type === 'error' ? 'error' : 'success'}`;
    alert.innerHTML = `
        <span>${escapeHtml(message)}</span>
        <button onclick="this.parentElement.remove()" style="background:none;border:none;cursor:pointer;font-weight:bold;">&times;</button>
    `;
    container.appendChild(alert);

    if (durationMs > 0) {
        setTimeout(() => {
            if (alert.parentElement) alert.remove();
        }, durationMs);
    }
}

function escapeHtml(str) {
    if (!str) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}

// Mocks Management
async function loadMocks() {
    const tbody = document.getElementById('mocks-table-body');
    try {
        const response = await fetch('/__admin/mocks');
        if (!response.ok) {
            throw new Error(`Failed to load mocks (HTTP ${response.status})`);
        }
        mocksCache = await response.json();
        renderMocksTable();
    } catch (err) {
        tbody.innerHTML = `<tr><td colspan="6" class="text-center text-muted">Error: ${escapeHtml(err.message)}</td></tr>`;
        showAlert(err.message, 'error');
    }
}

function renderMocksTable() {
    const tbody = document.getElementById('mocks-table-body');
    const badge = document.getElementById('mock-count-badge');

    badge.textContent = `${mocksCache.length} mock${mocksCache.length === 1 ? '' : 's'}`;

    if (!mocksCache || mocksCache.length === 0) {
        tbody.innerHTML = `<tr><td colspan="6" class="text-center text-muted">No mocks registered.</td></tr>`;
        return;
    }

    tbody.innerHTML = mocksCache.map(mock => {
        const methodClass = getMethodBadgeClass(mock.method);
        const typeClass = getTypeBadgeClass(mock.type);

        const resetButton = (mock.type === 'sequence' || mock.type === 'rest')
            ? `<button onclick="resetMockState('${mock.id}')" class="btn btn-xs btn-warning">Reset</button>`
            : '';

        return `
            <tr>
                <td><span class="badge ${methodClass}">${escapeHtml(mock.method)}</span></td>
                <td><code>${escapeHtml(mock.path)}</code></td>
                <td><span class="badge ${typeClass}">${escapeHtml(mock.type)}</span></td>
                <td>${mock.priority}</td>
                <td><strong>${escapeHtml(mock.name)}</strong></td>
                <td>
                    <div style="display:flex; gap:6px;">
                        <button onclick="editMock('${mock.id}')" class="btn btn-xs btn-primary">Edit</button>
                        ${resetButton}
                        <button onclick="deleteMock('${mock.id}')" class="btn btn-xs btn-danger">Delete</button>
                    </div>
                </td>
            </tr>
        `;
    }).join('');
}

function getMethodBadgeClass(method) {
    switch (method.toUpperCase()) {
        case 'GET': return 'badge-method-get';
        case 'POST': return 'badge-method-post';
        case 'PUT': return 'badge-method-put';
        case 'DELETE': return 'badge-method-delete';
        case 'PATCH': return 'badge-method-patch';
        default: return 'badge-method-all';
    }
}

function getTypeBadgeClass(type) {
    switch (type) {
        case 'static': return 'badge-success';
        case 'conditional': return 'badge-info';
        case 'sequence': return 'badge-warning';
        case 'script': return 'badge-purple';
        case 'rest': return 'badge-primary';
        default: return 'badge-secondary';
    }
}

function onMockTypeChange() {
    const type = document.getElementById('mock-type').value;
    const panels = ['static', 'conditional', 'sequence', 'script', 'rest'];
    panels.forEach(p => {
        const el = document.getElementById(`panel-${p}`);
        if (el) el.style.display = (p === type) ? 'block' : 'none';
    });
}

function addConditionItem(data = null) {
    conditionsState.push(data || {
        source: 'header',
        key: '',
        op: 'equals',
        value: '',
        response: { status: 200, contentType: 'application/json', body: '' }
    });
    renderConditions();
}

function removeConditionItem(index) {
    conditionsState.splice(index, 1);
    renderConditions();
}

function renderConditions() {
    const container = document.getElementById('conditions-list');
    if (conditionsState.length === 0) {
        container.innerHTML = `<p class="text-muted" style="font-size:0.85rem;">No conditions added.</p>`;
        return;
    }

    container.innerHTML = conditionsState.map((c, idx) => `
        <div class="dynamic-item">
            <select onchange="conditionsState[${idx}].source = this.value" class="form-control" style="width:110px;">
                <option value="header" ${c.source === 'header' ? 'selected' : ''}>Header</option>
                <option value="query" ${c.source === 'query' ? 'selected' : ''}>Query</option>
                <option value="body" ${c.source === 'body' ? 'selected' : ''}>Body</option>
            </select>
            <input type="text" placeholder="Key/Header" value="${escapeHtml(c.key)}" oninput="conditionsState[${idx}].key = this.value" class="form-control" style="flex:1;">
            <select onchange="conditionsState[${idx}].op = this.value" class="form-control" style="width:120px;">
                <option value="equals" ${c.op === 'equals' ? 'selected' : ''}>equals</option>
                <option value="notEquals" ${c.op === 'notEquals' ? 'selected' : ''}>notEquals</option>
                <option value="exists" ${c.op === 'exists' ? 'selected' : ''}>exists</option>
                <option value="notExists" ${c.op === 'notExists' ? 'selected' : ''}>notExists</option>
            </select>
            <input type="text" placeholder="Expected value" value="${escapeHtml(c.value || '')}" oninput="conditionsState[${idx}].value = this.value" class="form-control" style="flex:1;">
            <input type="number" placeholder="Status" value="${c.response ? c.response.status : 200}" oninput="if(!conditionsState[${idx}].response) conditionsState[${idx}].response={}; conditionsState[${idx}].response.status = parseInt(this.value)||200" class="form-control" style="width:80px;">
            <input type="text" placeholder="Body" value="${escapeHtml(c.response ? c.response.body : '')}" oninput="if(!conditionsState[${idx}].response) conditionsState[${idx}].response={}; conditionsState[${idx}].response.body = this.value" class="form-control" style="flex:1;">
            <button type="button" onclick="removeConditionItem(${idx})" class="btn btn-xs btn-danger">&times;</button>
        </div>
    `).join('');
}

function addSequenceItem(data = null) {
    sequenceState.push(data || { status: 200, contentType: 'application/json', body: '' });
    renderSequenceItems();
}

function removeSequenceItem(index) {
    sequenceState.splice(index, 1);
    renderSequenceItems();
}

function renderSequenceItems() {
    const container = document.getElementById('sequence-list');
    if (sequenceState.length === 0) {
        container.innerHTML = `<p class="text-muted" style="font-size:0.85rem;">No items added to sequence.</p>`;
        return;
    }

    container.innerHTML = sequenceState.map((item, idx) => `
        <div class="dynamic-item">
            <span style="font-weight:bold; font-size:0.8rem; width:20px;">#${idx + 1}</span>
            <input type="number" placeholder="Status" value="${item.status || 200}" oninput="sequenceState[${idx}].status = parseInt(this.value)||200" class="form-control" style="width:80px;">
            <input type="text" placeholder="Content-Type" value="${escapeHtml(item.contentType || '')}" oninput="sequenceState[${idx}].contentType = this.value" class="form-control" style="width:140px;">
            <input type="text" placeholder="Body" value="${escapeHtml(item.body || '')}" oninput="sequenceState[${idx}].body = this.value" class="form-control" style="flex:1;">
            <button type="button" onclick="removeSequenceItem(${idx})" class="btn btn-xs btn-danger">&times;</button>
        </div>
    `).join('');
}

function editMock(id) {
    const mock = mocksCache.find(m => m.id === id);
    if (!mock) return;

    editingMockId = id;
    document.getElementById('form-title').textContent = `Edit Mock (${mock.name})`;
    document.getElementById('btn-cancel-edit').style.display = 'inline-block';
    document.getElementById('mock-id').value = mock.id;
    document.getElementById('mock-name').value = mock.name;
    document.getElementById('mock-method').value = mock.method;
    document.getElementById('mock-path').value = mock.path;
    document.getElementById('mock-priority').value = mock.priority || 0;
    document.getElementById('mock-type').value = mock.type;
    onMockTypeChange();

    if (mock.type === 'static' && mock.response) {
        document.getElementById('static-status').value = mock.response.status || 200;
        document.getElementById('static-content-type').value = mock.response.contentType || '';
        document.getElementById('static-body').value = mock.response.body || '';
    } else if (mock.type === 'conditional') {
        conditionsState = mock.conditions ? JSON.parse(JSON.stringify(mock.conditions)) : [];
        renderConditions();
        if (mock.elseResponse) {
            document.getElementById('else-status').value = mock.elseResponse.status || '';
            document.getElementById('else-body').value = mock.elseResponse.body || '';
        }
    } else if (mock.type === 'sequence') {
        sequenceState = mock.sequence ? JSON.parse(JSON.stringify(mock.sequence)) : [];
        renderSequenceItems();
        if (mock.sequenceMode) {
            document.getElementById('sequence-mode').value = mock.sequenceMode;
        }
    } else if (mock.type === 'script') {
        document.getElementById('script-code').value = mock.script || '';
        document.getElementById('script-timeout').value = mock.scriptTimeoutMs || '';
    } else if (mock.type === 'rest') {
        document.getElementById('rest-id-field').value = mock.restIdField || 'id';
        document.getElementById('rest-initial-state').value = mock.restInitialState
            ? JSON.stringify(mock.restInitialState, null, 2)
            : '';
    }

    document.getElementById('form-card').scrollIntoView({ behavior: 'smooth' });
}

function resetForm() {
    editingMockId = null;
    document.getElementById('form-title').textContent = 'Create New Mock';
    document.getElementById('btn-cancel-edit').style.display = 'none';
    document.getElementById('mock-form').reset();
    document.getElementById('mock-id').value = '';
    document.getElementById('mock-type').value = 'static';
    onMockTypeChange();
    conditionsState = [];
    renderConditions();
    sequenceState = [];
    renderSequenceItems();
}

async function handleFormSubmit(e) {
    e.preventDefault();

    const name = document.getElementById('mock-name').value.trim();
    const method = document.getElementById('mock-method').value;
    const path = document.getElementById('mock-path').value.trim();
    const priority = parseInt(document.getElementById('mock-priority').value, 10) || 0;
    const type = document.getElementById('mock-type').value;

    const mockPayload = {
        name,
        method,
        path,
        priority,
        type
    };

    if (type === 'static') {
        mockPayload.response = {
            status: parseInt(document.getElementById('static-status').value, 10) || 200,
            contentType: document.getElementById('static-content-type').value.trim() || undefined,
            body: document.getElementById('static-body').value
        };
    } else if (type === 'conditional') {
        mockPayload.conditions = conditionsState;
        const elseStatus = parseInt(document.getElementById('else-status').value, 10);
        const elseBody = document.getElementById('else-body').value;
        if (elseStatus || elseBody) {
            mockPayload.elseResponse = {
                status: elseStatus || 404,
                body: elseBody
            };
        }
    } else if (type === 'sequence') {
        mockPayload.sequence = sequenceState;
        mockPayload.sequenceMode = document.getElementById('sequence-mode').value;
    } else if (type === 'script') {
        mockPayload.script = document.getElementById('script-code').value;
        const timeout = parseInt(document.getElementById('script-timeout').value, 10);
        if (timeout) mockPayload.scriptTimeoutMs = timeout;
    } else if (type === 'rest') {
        mockPayload.restIdField = document.getElementById('rest-id-field').value.trim() || 'id';
        const rawState = document.getElementById('rest-initial-state').value.trim();
        if (rawState) {
            try {
                mockPayload.restInitialState = JSON.parse(rawState);
            } catch (err) {
                showAlert('Invalid JSON in REST mock Initial State: ' + err.message, 'error');
                return;
            }
        }
    }

    try {
        let response;
        if (editingMockId) {
            response = await fetch(`/__admin/mocks/${editingMockId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(mockPayload)
            });
        } else {
            response = await fetch('/__admin/mocks', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(mockPayload)
            });
        }

        if (!response.ok) {
            const errData = await response.json().catch(() => ({}));
            throw new Error(errData.message || `HTTP Error ${response.status}`);
        }

        showAlert(editingMockId ? 'Mock updated successfully!' : 'Mock created successfully!');
        resetForm();
        loadMocks();
    } catch (err) {
        showAlert('Failed to save mock: ' + err.message, 'error');
    }
}

async function deleteMock(id) {
    if (!confirm('Are you sure you want to delete this mock?')) return;
    try {
        const response = await fetch(`/__admin/mocks/${id}`, { method: 'DELETE' });
        if (response.ok) {
            showAlert('Mock deleted successfully!');
            if (editingMockId === id) resetForm();
            loadMocks();
        } else {
            const errData = await response.json().catch(() => ({}));
            throw new Error(errData.message || `HTTP Error ${response.status}`);
        }
    } catch (err) {
        showAlert('Failed to delete mock: ' + err.message, 'error');
    }
}

async function resetMockState(id) {
    try {
        const response = await fetch(`/__admin/mocks/${id}/reset`, { method: 'POST' });
        if (response.ok) {
            showAlert('State/sequence reset successfully!');
        } else {
            const errData = await response.json().catch(() => ({}));
            throw new Error(errData.message || `HTTP Error ${response.status}`);
        }
    } catch (err) {
        showAlert('Failed to reset mock: ' + err.message, 'error');
    }
}

// LiveFeed SSE
function initLiveFeedSSE() {
    const sseStatus = document.getElementById('sse-status');

    if (eventSource) {
        eventSource.close();
    }

    eventSource = new EventSource('/__admin/livefeed');

    eventSource.onopen = () => {
        if (sseStatus) {
            sseStatus.className = 'badge badge-success';
            sseStatus.textContent = '🟢 Connected';
        }
    };

    eventSource.onmessage = (event) => {
        try {
            const entry = JSON.parse(event.data);
            liveFeedEvents.unshift(entry);
            if (liveFeedEvents.length > 100) liveFeedEvents.pop();
            renderLiveFeedTable();
        } catch (e) {
            console.error('Error processing SSE:', e);
        }
    };

    eventSource.onerror = () => {
        if (sseStatus) {
            sseStatus.className = 'badge badge-danger';
            sseStatus.textContent = '🔴 Disconnected';
        }
    };
}

function renderLiveFeedTable() {
    const tbody = document.getElementById('feed-table-body');
    if (!liveFeedEvents || liveFeedEvents.length === 0) {
        tbody.innerHTML = `<tr><td colspan="6" class="text-center text-muted">Waiting for requests...</td></tr>`;
        return;
    }

    tbody.innerHTML = liveFeedEvents.map(e => {
        const methodClass = getMethodBadgeClass(e.method);
        const statusClass = e.status >= 200 && e.status < 300 ? 'badge-success' : (e.status >= 400 ? 'badge-danger' : 'badge-warning');
        const matchedText = e.matchedMockName
            ? `<strong>${escapeHtml(e.matchedMockName)}</strong>`
            : `<span class="text-muted">(No match)</span>`;
        const time = new Date(e.timestamp).toLocaleTimeString();

        return `
            <tr>
                <td><small>${time}</small></td>
                <td><span class="badge ${methodClass}">${escapeHtml(e.method)}</span></td>
                <td><code>${escapeHtml(e.path)}</code></td>
                <td>${matchedText}</td>
                <td><span class="badge ${statusClass}">${e.status}</span></td>
                <td><small>${e.durationMs.toFixed(2)} ms</small></td>
            </tr>
        `;
    }).join('');
}

function clearFeed() {
    liveFeedEvents = [];
    renderLiveFeedTable();
}
