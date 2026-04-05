import { providerDefaults } from './state.js';
import { state } from './state.js';

let currentConfig = null;
let editingIndex = null; // null = add mode, number = edit mode

export async function loadConfigIntoForm() {
    try {
        const res = await fetch('/api/config');
        const cfg = await res.json();
        currentConfig = cfg;
        state.cachedConfig = cfg;
        renderModelList(cfg.models || [], cfg.default_model || null);
    } catch (e) {
        console.error('Failed to load config:', e);
    }
}

function renderModelList(models, defaultModel) {
    const container = document.getElementById('model-list');
    if (!container) return;
    container.innerHTML = '';

    if (!models || models.length === 0) {
        container.innerHTML = '<div style="color:var(--foreground-secondary);font-size:13px;padding:8px 0;">No models configured. Click "+ Add Model" to add one.</div>';
        return;
    }

    models.forEach((m, idx) => {
        const isDefault = m.name === defaultModel;
        const item = document.createElement('div');
        item.className = 'model-item';

        // Info section
        const info = document.createElement('div');
        info.className = 'model-item-info';
        const nameSpan = document.createElement('span');
        nameSpan.className = 'model-item-name';
        nameSpan.textContent = m.name;
        if (isDefault) {
            const badge = document.createElement('span');
            badge.className = 'model-default-badge';
            badge.textContent = 'Default';
            nameSpan.appendChild(document.createTextNode(' '));
            nameSpan.appendChild(badge);
        }
        const detailSpan = document.createElement('span');
        detailSpan.className = 'model-item-detail';
        detailSpan.textContent = `${m.provider || ''} · ${m.model || ''}`;
        info.appendChild(nameSpan);
        info.appendChild(detailSpan);

        // Actions section
        const actions = document.createElement('div');
        actions.className = 'model-item-actions';
        if (!isDefault) {
            const defaultBtn = document.createElement('button');
            defaultBtn.className = 'model-item-default-btn';
            defaultBtn.title = 'Set as default';
            defaultBtn.textContent = 'Set Default';
            defaultBtn.addEventListener('click', () => setDefault(idx));
            actions.appendChild(defaultBtn);
        }
        const editBtn = document.createElement('button');
        editBtn.className = 'model-item-edit-btn';
        editBtn.title = 'Edit';
        editBtn.textContent = 'Edit';
        editBtn.addEventListener('click', () => openEditModelModal(idx));
        actions.appendChild(editBtn);
        const delBtn = document.createElement('button');
        delBtn.className = 'model-item-delete';
        delBtn.title = 'Delete';
        delBtn.innerHTML = '&times;';
        delBtn.addEventListener('click', () => deleteModel(idx));
        actions.appendChild(delBtn);

        item.appendChild(info);
        item.appendChild(actions);
        container.appendChild(item);
    });
}

export function getModels() {
    return currentConfig?.models || [];
}

export function getDefaultModelName() {
    return currentConfig?.default_model || null;
}

export function initSettings() {
    // Add Model modal
    document.getElementById('add-model-btn')?.addEventListener('click', openAddModelModal);
    document.getElementById('model-entry-cancel')?.addEventListener('click', () => closeModelModal());
    document.getElementById('model-modal-backdrop')?.addEventListener('click', () => closeModelModal());
    document.getElementById('model-entry-save')?.addEventListener('click', confirmModelModal);

    // Provider auto-fill for model modal
    document.getElementById('model-entry-provider')?.addEventListener('change', (e) => {
        const defaults = providerDefaults[e.target.value];
        if (defaults) {
            document.getElementById('model-entry-url').value = defaults.url;
            document.getElementById('model-entry-model').value = defaults.model;
        }
    });

    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') closeModelModal();
    });
}

function openAddModelModal() {
    editingIndex = null;
    document.getElementById('model-entry-name').value = '';
    document.getElementById('model-entry-provider').value = '';
    document.getElementById('model-entry-url').value = '';
    document.getElementById('model-entry-key').value = '';
    document.getElementById('model-entry-model').value = '';
    document.querySelector('#add-model-modal .modal-title').textContent = 'Add Model';
    document.getElementById('add-model-modal').style.display = 'flex';
    document.getElementById('model-entry-name').focus();
}

function openEditModelModal(idx) {
    const m = currentConfig.models[idx];
    if (!m) return;
    editingIndex = idx;
    document.getElementById('model-entry-name').value = m.name;
    document.getElementById('model-entry-provider').value = m.provider || '';
    document.getElementById('model-entry-url').value = m.api_url || '';
    document.getElementById('model-entry-key').value = m.api_key || '';
    document.getElementById('model-entry-model').value = m.model || '';
    document.querySelector('#add-model-modal .modal-title').textContent = 'Edit Model';
    document.getElementById('add-model-modal').style.display = 'flex';
    document.getElementById('model-entry-name').focus();
}

function closeModelModal() {
    document.getElementById('add-model-modal').style.display = 'none';
    editingIndex = null;
}

async function saveConfig() {
    await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(currentConfig),
    });
    state.cachedConfig = null;
}

async function confirmModelModal() {
    const name = document.getElementById('model-entry-name').value.trim();
    if (!name) { alert('Model name is required.'); return; }

    const entry = {
        name,
        provider: document.getElementById('model-entry-provider').value,
        api_url: document.getElementById('model-entry-url').value,
        api_key: document.getElementById('model-entry-key').value,
        model: document.getElementById('model-entry-model').value,
    };

    if (editingIndex !== null) {
        // Edit mode
        const oldName = currentConfig.models[editingIndex].name;
        // Check duplicate name (excluding self)
        if ((currentConfig.models || []).some((m, i) => i !== editingIndex && m.name === name)) {
            alert('A model with this name already exists.');
            return;
        }
        currentConfig.models[editingIndex] = entry;
        // Update default_model reference if name changed
        if (currentConfig.default_model === oldName) {
            currentConfig.default_model = name;
        }
    } else {
        // Add mode
        if ((currentConfig.models || []).some(m => m.name === name)) {
            alert('A model with this name already exists.');
            return;
        }
        if (!currentConfig.models) currentConfig.models = [];
        const isFirst = currentConfig.models.length === 0;
        currentConfig.models.push(entry);
        if (isFirst) {
            currentConfig.default_model = name;
        }
    }

    await saveConfig();
    renderModelList(currentConfig.models, currentConfig.default_model);
    closeModelModal();

    const fb = document.getElementById('model-save-feedback');
    fb.classList.add('show');
    setTimeout(() => fb.classList.remove('show'), 2000);
}

async function deleteModel(idx) {
    const removed = currentConfig.models[idx];
    if (!confirm(`Delete model "${removed.name}"?`)) return;
    currentConfig.models.splice(idx, 1);

    // If deleting the default, clear or reassign
    if (currentConfig.default_model === removed.name) {
        if (currentConfig.models.length > 0) {
            currentConfig.default_model = currentConfig.models[0].name;
        } else {
            currentConfig.default_model = null;
        }
    }

    await saveConfig();
    renderModelList(currentConfig.models, currentConfig.default_model);
}

async function setDefault(idx) {
    const entry = currentConfig.models[idx];
    currentConfig.default_model = entry.name;

    await saveConfig();
    renderModelList(currentConfig.models, currentConfig.default_model);
}
