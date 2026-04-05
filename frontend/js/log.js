let cachedLogs = [];

export async function loadLogs() {
    const container = document.getElementById('log-list');
    if (!container) return;

    try {
        const res = await fetch('/api/logs');
        const data = await res.json();
        cachedLogs = data.logs || [];
        renderLogs();
    } catch (e) {
        container.innerHTML = '<div class="log-empty">Failed to load logs.</div>';
    }
}

function renderLogs(filter = '') {
    const container = document.getElementById('log-list');
    if (!container) return;

    let logs = cachedLogs;
    const keyword = filter.trim().toLowerCase();
    if (keyword) {
        logs = logs.filter(e =>
            e.agent.toLowerCase().includes(keyword) ||
            e.path.toLowerCase().includes(keyword) ||
            e.action.toLowerCase().includes(keyword)
        );
    }

    container.innerHTML = '';

    if (logs.length === 0) {
        container.innerHTML = '<div class="log-empty">' + (keyword ? 'No matching logs.' : 'No activity yet. Agent file modifications will appear here.') + '</div>';
        return;
    }

    logs.forEach(entry => {
        const row = document.createElement('div');
        row.className = 'log-entry';

        const dot = document.createElement('span');
        dot.className = 'log-dot';

        const agent = document.createElement('span');
        agent.className = 'log-agent';
        agent.textContent = entry.agent;

        const action = document.createElement('span');
        action.className = 'log-action';
        if (entry.action === 'write_file') {
            action.textContent = 'modified';
        } else if (entry.action === 'exec') {
            const cmd = entry.path || '';
            if (cmd.includes('mkdir')) {
                action.textContent = 'created folder';
            } else if (cmd.includes('touch') || cmd.includes('cat >') || cmd.includes('> ')) {
                action.textContent = 'created file';
            } else {
                action.textContent = 'ran command';
            }
        } else {
            action.textContent = entry.action;
        }

        const path = document.createElement('span');
        path.className = 'log-path';
        path.textContent = entry.path;

        const time = document.createElement('span');
        time.className = 'log-time';
        time.textContent = formatTime(entry.timestamp);

        row.appendChild(dot);
        row.appendChild(agent);
        row.appendChild(action);
        row.appendChild(path);
        row.appendChild(time);
        container.appendChild(row);
    });
}

function formatTime(iso) {
    try {
        const match = iso.match(/T(\d{2}):(\d{2})/);
        if (match) {
            const date = iso.match(/(\d{4}-\d{2}-\d{2})/)?.[1] || '';
            return `${date} ${match[1]}:${match[2]}`;
        }
    } catch {}
    return iso;
}

export function initLog() {
    document.getElementById('log-refresh-btn')?.addEventListener('click', loadLogs);

    const searchInput = document.getElementById('log-search');
    if (searchInput) {
        searchInput.addEventListener('input', () => {
            renderLogs(searchInput.value);
        });
    }
}
