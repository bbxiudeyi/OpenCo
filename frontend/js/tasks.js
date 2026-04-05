import { state } from './state.js';

let cachedTasks = [];

export async function loadTasks() {
    try {
        const res = await fetch('/api/tasks');
        const data = await res.json();
        cachedTasks = data.tasks || [];
        renderTasks();
    } catch (e) {
        console.error('Failed to load tasks:', e);
    }
}

function renderTasks() {
    const container = document.getElementById('task-list');
    if (!container) return;
    container.innerHTML = '';

    // Only show in_progress tasks
    const inProgress = cachedTasks.filter(t => t.status === 'in_progress');

    if (inProgress.length === 0) {
        container.innerHTML = '<div class="task-empty">No active tasks. Click "+ New Task" to create one.</div>';
        return;
    }

    inProgress.forEach(task => {
        const card = document.createElement('div');
        card.className = 'task-card';

        // Header row: title + action buttons
        const header = document.createElement('div');
        header.className = 'task-card-header';

        const title = document.createElement('div');
        title.className = 'task-card-title';
        title.textContent = task.title;

        const actions = document.createElement('div');
        actions.className = 'task-card-actions';

        const editBtn = document.createElement('button');
        editBtn.textContent = 'Edit';
        editBtn.addEventListener('click', (e) => { e.stopPropagation(); openEditTaskModal(task); });

        const doneBtn = document.createElement('button');
        doneBtn.textContent = 'Done';
        doneBtn.addEventListener('click', (e) => { e.stopPropagation(); markDone(task.id); });

        const deleteBtn = document.createElement('button');
        deleteBtn.textContent = '×';
        deleteBtn.className = 'task-delete-btn';
        deleteBtn.addEventListener('click', (e) => { e.stopPropagation(); deleteTask(task.id); });

        actions.appendChild(editBtn);
        actions.appendChild(doneBtn);
        actions.appendChild(deleteBtn);
        header.appendChild(title);
        header.appendChild(actions);
        card.appendChild(header);

        // Description
        if (task.description) {
            const desc = document.createElement('div');
            desc.className = 'task-card-desc';
            desc.textContent = task.description;
            card.appendChild(desc);
        }

        // Agent tags
        const agentsDiv = document.createElement('div');
        agentsDiv.className = 'task-card-agents';
        if (task.agents && task.agents.length > 0) {
            task.agents.forEach(name => {
                const tag = document.createElement('span');
                tag.className = 'task-agent-tag';
                tag.textContent = name;
                agentsDiv.appendChild(tag);
            });
        } else {
            const empty = document.createElement('span');
            empty.className = 'task-agent-empty';
            empty.textContent = 'No agents assigned';
            agentsDiv.appendChild(empty);
        }
        card.appendChild(agentsDiv);

        // Progress bar
        const progressWrap = document.createElement('div');
        progressWrap.className = 'task-progress-wrap';

        const bar = document.createElement('div');
        bar.className = 'task-progress-bar';
        const fill = document.createElement('div');
        fill.className = 'task-progress-fill' + (task.progress >= 100 ? ' complete' : '');
        fill.style.width = (task.progress || 0) + '%';
        bar.appendChild(fill);

        const label = document.createElement('span');
        label.className = 'task-progress-label';
        label.textContent = (task.progress || 0) + '%';

        progressWrap.appendChild(bar);
        progressWrap.appendChild(label);
        card.appendChild(progressWrap);

        container.appendChild(card);
    });
}

// --- Modal ---
function openAddTaskModal() {
    document.getElementById('task-modal-title').textContent = 'New Task';
    document.getElementById('task-entry-id').value = '';
    document.getElementById('task-entry-title').value = '';
    document.getElementById('task-entry-desc').value = '';
    document.getElementById('add-task-modal').style.display = 'flex';
    document.getElementById('task-entry-title').focus();
}

function openEditTaskModal(task) {
    document.getElementById('task-modal-title').textContent = 'Edit Task';
    document.getElementById('task-entry-id').value = task.id;
    document.getElementById('task-entry-title').value = task.title;
    document.getElementById('task-entry-desc').value = task.description || '';
    document.getElementById('add-task-modal').style.display = 'flex';
}

function closeTaskModal() {
    document.getElementById('add-task-modal').style.display = 'none';
}

async function saveTaskModal() {
    const id = document.getElementById('task-entry-id').value;
    const title = document.getElementById('task-entry-title').value.trim();
    const desc = document.getElementById('task-entry-desc').value.trim();

    if (!title) { alert('Title is required.'); return; }

    if (id) {
        // Update existing
        await fetch(`/api/tasks/${id}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ title, description: desc }),
        });
    } else {
        // Create new
        await fetch('/api/tasks', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ title, description: desc }),
        });
    }

    closeTaskModal();
    await loadTasks();
}

async function markDone(id) {
    await fetch(`/api/tasks/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ status: 'done', progress: 100 }),
    });
    await loadTasks();
}

async function deleteTask(id) {
    if (!confirm('Delete this task?')) return;
    await fetch(`/api/tasks/${id}`, { method: 'DELETE' });
    await loadTasks();
}

export function initTasks() {
    document.getElementById('add-task-btn')?.addEventListener('click', openAddTaskModal);
    document.getElementById('task-entry-cancel')?.addEventListener('click', closeTaskModal);
    document.getElementById('task-modal-backdrop')?.addEventListener('click', closeTaskModal);
    document.getElementById('task-entry-save')?.addEventListener('click', saveTaskModal);

    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            const modal = document.getElementById('add-task-modal');
            if (modal && modal.style.display !== 'none') closeTaskModal();
        }
    });
}
