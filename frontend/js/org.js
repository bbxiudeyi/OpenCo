import { state } from './state.js';
import { getConfig } from './chat.js';
import { loadAgents, renderAgentList } from './agents.js';
import { getModels } from './settings.js';

// --- Zoom & Pan ---
function applyTransform() {
    const tree = document.getElementById('org-tree');
    if (!tree) return;
    tree.style.transform = `translate(${state.panX}px, ${state.panY}px) scale(${state.currentZoom})`;
    const label = document.getElementById('zoom-label');
    if (label) label.textContent = Math.round(state.currentZoom * 100) + '%';
}

function applyZoom(scale) {
    state.currentZoom = Math.min(2, Math.max(0.3, scale));
    applyTransform();
}

function resetView() {
    state.currentZoom = 1;
    state.panX = 0;
    state.panY = 0;
    applyTransform();
}

function initZoom() {
    document.getElementById('zoom-in')?.addEventListener('click', () => applyZoom(state.currentZoom + 0.1));
    document.getElementById('zoom-out')?.addEventListener('click', () => applyZoom(state.currentZoom - 0.1));
    document.getElementById('zoom-reset')?.addEventListener('click', resetView);

    const manageBody = document.getElementById('manage-body');
    if (manageBody) {
        manageBody.addEventListener('wheel', e => {
            if (e.ctrlKey || e.metaKey) {
                e.preventDefault();
                const delta = e.deltaY > 0 ? -0.1 : 0.1;
                applyZoom(state.currentZoom + delta);
            }
        }, { passive: false });
    }

    const treeWrapper = document.getElementById('org-tree-wrapper');
    if (treeWrapper) {
        treeWrapper.addEventListener('wheel', e => {
            e.preventDefault();
            const delta = e.deltaY > 0 ? -0.08 : 0.08;
            applyZoom(state.currentZoom + delta);
        }, { passive: false });

        // Drag to pan
        let isDragging = false;
        let startX = 0, startY = 0;

        treeWrapper.addEventListener('mousedown', e => {
            if (e.target.closest('.org-node')) return;
            isDragging = true;
            startX = e.clientX - state.panX;
            startY = e.clientY - state.panY;
            treeWrapper.style.cursor = 'grabbing';
            e.preventDefault();
        });

        window.addEventListener('mousemove', e => {
            if (!isDragging) return;
            state.panX = e.clientX - startX;
            state.panY = e.clientY - startY;
            applyTransform();
        });

        window.addEventListener('mouseup', () => {
            if (isDragging) {
                isDragging = false;
                if (treeWrapper) treeWrapper.style.cursor = 'grab';
            }
        });

        treeWrapper.style.cursor = 'grab';
    }
}

// --- Modal ---
function openAddModal() {
    const modal = document.getElementById('add-pos-modal');
    if (!modal) return;

    const parentSelect = document.getElementById('modal-pos-parent');
    parentSelect.innerHTML = '<option value="">-- None (Top Level) --</option>';
    state.orgData.positions.forEach(p => {
        const opt = document.createElement('option');
        opt.value = p.id;
        const label = p.agents?.length ? p.agents.join(', ') : p.title;
        opt.textContent = label;
        if (state.selectedPositionId && p.id === state.selectedPositionId) opt.selected = true;
        parentSelect.appendChild(opt);
    });

    document.getElementById('modal-pos-title').value = '';
    document.getElementById('modal-agent-name').value = '';

    // Populate model dropdown
    const modelSelect = document.getElementById('modal-agent-model');
    modelSelect.innerHTML = '<option value="">-- Use Default --</option>';
    const models = getModels();
    models.forEach(m => {
        const opt = document.createElement('option');
        opt.value = m.name;
        opt.textContent = m.name;
        modelSelect.appendChild(opt);
    });

    modal.style.display = 'flex';
    document.getElementById('modal-agent-name').focus();
}

function closeAddModal(force) {
    const titleVal = document.getElementById('modal-pos-title')?.value?.trim();
    const agentVal = document.getElementById('modal-agent-name')?.value?.trim();
    const hasContent = titleVal || agentVal;

    if (!force && hasContent) {
        if (!confirm('Discard unsaved position?')) return;
    }
    document.getElementById('add-pos-modal').style.display = 'none';
}

async function confirmAddModal() {
    const title = document.getElementById('modal-pos-title').value.trim();
    if (!title) { alert('Position title is required.'); return; }

    const parentId = document.getElementById('modal-pos-parent').value || null;
    const agentName = document.getElementById('modal-agent-name').value.trim();

    const body = { title, parent_id: parentId };

    if (agentName) {
        const model_name = document.getElementById('modal-agent-model').value || null;
        body.create_agent = {
            name: agentName,
            model: '',
            api_url: '',
            api_key: '',
            system_prompt: '',
            tools: ['web_search', 'read_file', 'write_file', 'exec'],
            model_name,
        };
    }

    const res = await fetch('/api/org/positions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });

    const data = await res.json();
    if (data.error) {
        alert('Error: ' + data.error);
        return;
    }

    await loadOrg();
    await loadAgents();
    closeAddModal(true);

    if (data.position && data.position.id) {
        selectPosition(data.position.id);
    }
}

function initModal() {
    document.getElementById('org-add-btn')?.addEventListener('click', openAddModal);
    document.getElementById('modal-cancel')?.addEventListener('click', () => closeAddModal(false));
    document.getElementById('modal-backdrop')?.addEventListener('click', () => closeAddModal(false));
    document.getElementById('modal-confirm')?.addEventListener('click', confirmAddModal);

    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            const modal = document.getElementById('add-pos-modal');
            if (modal && modal.style.display !== 'none') closeAddModal(false);
        }
    });
}

// --- Org Data ---
export async function loadOrg() {
    try {
        const res = await fetch('/api/org');
        state.orgData = await res.json();
        renderOrgTree();
        renderAgentList();
    } catch (e) {
        console.error('Failed to load org:', e);
    }
}

function renderOrgTree() {
    const container = document.getElementById('org-tree');
    if (!container) return;
    container.innerHTML = '';

    if (!state.orgData.positions || state.orgData.positions.length === 0) {
        container.innerHTML = `
            <div class="org-empty">
                <div class="org-empty-text">No positions yet</div>
                <div class="org-empty-hint">Click "+ Add Position" to create your company structure</div>
            </div>`;
        return;
    }

    const byParent = {};
    const roots = [];
    state.orgData.positions.forEach(p => {
        if (!p.parent_id) {
            roots.push(p);
        } else {
            if (!byParent[p.parent_id]) byParent[p.parent_id] = [];
            byParent[p.parent_id].push(p);
        }
    });

    function renderLevel(nodes) {
        const levelDiv = document.createElement('div');
        levelDiv.className = 'org-level';
        nodes.forEach(pos => {
            const node = document.createElement('div');
            node.className = 'org-node' + (state.selectedPositionId === pos.id ? ' selected' : '');
            node.dataset.id = pos.id;
            const titleDiv = document.createElement('div');
            titleDiv.className = 'org-node-title';
            titleDiv.textContent = pos.title;
            const agentsDiv = document.createElement('div');
            agentsDiv.className = pos.agents?.length ? 'org-node-agents' : 'org-node-empty';
            agentsDiv.textContent = pos.agents?.length ? pos.agents.join(', ') : 'No agent';
            node.appendChild(titleDiv);
            node.appendChild(agentsDiv);
            node.addEventListener('click', (e) => { e.stopPropagation(); selectPosition(pos.id); });
            levelDiv.appendChild(node);
        });
        return levelDiv;
    }

    function renderTree(parentNodes) {
        const levelDiv = renderLevel(parentNodes);

        const wrapper = document.createElement('div');
        wrapper.className = 'org-branch';
        wrapper.appendChild(levelDiv);

        // Check if any parent has children
        const hasChildren = parentNodes.some(p => byParent[p.id]?.length > 0);
        if (!hasChildren) return wrapper;

        const childrenRow = document.createElement('div');
        childrenRow.className = 'org-children-row';
        parentNodes.forEach(parent => {
            const children = byParent[parent.id];
            if (children?.length > 0) {
                childrenRow.appendChild(renderTree(children));
            }
        });

        wrapper.appendChild(childrenRow);
        return wrapper;
    }

    roots.forEach(root => container.appendChild(renderTree([root])));
}

function selectPosition(id) {
    state.selectedPositionId = id;
    state.panelHasChanges = false;
    const pos = state.orgData.positions.find(p => p.id === id);
    if (!pos) return;

    const panel = document.getElementById('org-panel');
    panel.style.display = 'flex';

    // Agent name — show first assigned agent name
    const agentNameInput = document.getElementById('org-pos-agent-name');
    agentNameInput.value = pos.agents?.[0] || '';

    // Agent model — populate from settings model list
    const modelSelect = document.getElementById('org-pos-agent-model');
    modelSelect.innerHTML = '<option value="">-- Use Default --</option>';
    const models = getModels();
    let currentModelName = '';
    // Find the agent's current model_name from agents list
    if (pos.agents?.[0]) {
        const agent = state.agents.find(a => a.name === pos.agents[0]);
        if (agent) currentModelName = agent.model_name || '';
    }
    models.forEach(m => {
        const opt = document.createElement('option');
        opt.value = m.name;
        opt.textContent = m.name;
        if (m.name === currentModelName) opt.selected = true;
        modelSelect.appendChild(opt);
    });

    document.getElementById('org-pos-title').value = pos.title;
    document.getElementById('org-pos-prompt').value = pos.system_prompt || '';

    const parentSelect = document.getElementById('org-pos-parent');
    parentSelect.innerHTML = '<option value="">-- None (Top Level) --</option>';
    state.orgData.positions.forEach(p => {
        if (p.id !== id) {
            const opt = document.createElement('option');
            opt.value = p.id;
            const label = p.agents?.length ? p.agents.join(', ') : p.title;
            opt.textContent = label;
            if (pos.parent_id === p.id) opt.selected = true;
            parentSelect.appendChild(opt);
        }
    });

    // Track panel changes (bind once)
    ['org-pos-title', 'org-pos-prompt', 'org-pos-agent-name'].forEach(fieldId => {
        const el = document.getElementById(fieldId);
        if (el && !el.dataset.changeListener) {
            el.dataset.changeListener = 'true';
            el.addEventListener('input', () => { if (state.selectedPositionId) state.panelHasChanges = true; });
        }
    });
    ['org-pos-parent', 'org-pos-agent-model'].forEach(fieldId => {
        const el = document.getElementById(fieldId);
        if (el && !el.dataset.changeListener) {
            el.dataset.changeListener = 'true';
            el.addEventListener('change', () => { if (state.selectedPositionId) state.panelHasChanges = true; });
        }
    });

    renderOrgTree();
}

async function savePosition() {
    if (!state.selectedPositionId) return;

    const title = document.getElementById('org-pos-title').value.trim();
    const parentId = document.getElementById('org-pos-parent').value || null;
    const prompt = document.getElementById('org-pos-prompt').value;
    const agentName = document.getElementById('org-pos-agent-name').value.trim();
    const modelName = document.getElementById('org-pos-agent-model').value;

    const agents = agentName ? [agentName] : [];

    // Save position
    const res = await fetch(`/api/org/positions/${state.selectedPositionId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title, parent_id: parentId, agents, system_prompt: prompt }),
    });

    const data = await res.json();
    if (data.ok) {
        const pos = state.orgData.positions.find(p => p.id === state.selectedPositionId);
        if (pos) { pos.title = title; pos.parent_id = parentId; pos.agents = agents; pos.system_prompt = prompt; }

        // Update agent config with model_name
        if (agentName) {
            const agent = state.agents.find(a => a.name === agentName);
            if (agent) {
                const updatedAgent = { ...agent };
                if (modelName) {
                    updatedAgent.model_name = modelName;
                } else {
                    delete updatedAgent.model_name;
                }
                await fetch(`/api/agents/${agentName}/config`, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(updatedAgent),
                });
                // Update local state
                Object.assign(agent, updatedAgent);
            }
        }

        renderOrgTree();
        renderAgentList();
        state.panelHasChanges = false;
        const fb = document.getElementById('org-save-feedback');
        fb.classList.add('show');
        setTimeout(() => fb.classList.remove('show'), 2000);
    }
}

async function deletePosition() {
    if (!state.selectedPositionId) return;
    if (!confirm('Delete this position? Subordinates will be moved up.')) return;

    const res = await fetch(`/api/org/positions/${state.selectedPositionId}`, { method: 'DELETE' });
    const data = await res.json();
    if (data.ok) {
        state.orgData.positions = state.orgData.positions.filter(p => p.id !== state.selectedPositionId);
        state.selectedPositionId = null;
        state.panelHasChanges = false;
        document.getElementById('org-panel').style.display = 'none';
        renderOrgTree();
    }
}

export function initOrg() {
    initZoom();
    initModal();
    document.getElementById('org-pos-save')?.addEventListener('click', savePosition);
    document.getElementById('org-pos-delete')?.addEventListener('click', deletePosition);

    document.getElementById('manage-body')?.addEventListener('click', e => {
        const panel = document.getElementById('org-panel');
        if (!panel || panel.style.display === 'none') return;
        const clickedNode = e.target.closest('.org-node');
        const clickedPanel = e.target.closest('.org-panel');
        if (!clickedNode && !clickedPanel) {
            if (state.panelHasChanges) {
                if (!confirm('Discard unsaved changes?')) return;
            }
            panel.style.display = 'none';
            state.selectedPositionId = null;
            state.panelHasChanges = false;
            renderOrgTree();
        }
    });
}
