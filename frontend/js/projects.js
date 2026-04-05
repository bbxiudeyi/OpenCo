export async function loadProjects() {
    const grid = document.getElementById('projects-grid');
    if (!grid) return;

    try {
        const res = await fetch('/api/workspace/projects');
        const data = await res.json();
        const projects = data.projects || [];

        grid.innerHTML = '';

        if (projects.length === 0) {
            grid.innerHTML = '<div class="projects-empty">No projects yet. Click "+ New Project" to create one.</div>';
            return;
        }

        projects.forEach(name => {
            const card = document.createElement('div');
            card.className = 'project-card';
            card.innerHTML = `
                <div class="project-card-icon">
                    <i data-lucide="folder"></i>
                </div>
                <div class="project-card-name">${escapeHtml(name)}</div>
                <div class="project-card-actions">
                    <button class="btn-secondary btn-sm project-lookup-btn" data-name="${escapeAttr(name)}"><i data-lucide="external-link"></i> Look up</button>
                    <button class="btn-secondary btn-sm project-upload-btn" data-name="${escapeAttr(name)}"><i data-lucide="upload"></i> Upload</button>
                </div>
            `;
            grid.appendChild(card);
        });

        if (typeof lucide !== 'undefined') lucide.createIcons();
    } catch (e) {
        grid.innerHTML = '<div class="projects-empty">Failed to load projects.</div>';
    }
}

export function initProjects() {
    const grid = document.getElementById('projects-grid');
    if (!grid) return;

    grid.addEventListener('click', async (e) => {
        const lookupBtn = e.target.closest('.project-lookup-btn');
        if (lookupBtn) {
            const name = lookupBtn.dataset.name;
            try {
                const res = await fetch(`/api/workspace/projects/${encodeURIComponent(name)}/open`, { method: 'POST' });
                const data = await res.json();
                if (data.error) alert(data.error);
            } catch {
                alert('Failed to open project folder.');
            }
            return;
        }

        const uploadBtn = e.target.closest('.project-upload-btn');
        if (uploadBtn) {
            alert('Coming soon');
            return;
        }
    });

    const addBtn = document.getElementById('add-project-btn');
    if (addBtn) {
        addBtn.addEventListener('click', async () => {
            const name = prompt('Project name:');
            if (!name || !name.trim()) return;
            try {
                const res = await fetch('/api/workspace/projects', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ name: name.trim() }),
                });
                const data = await res.json();
                if (data.error) {
                    alert(data.error);
                } else {
                    await loadProjects();
                }
            } catch {
                alert('Failed to create project.');
            }
        });
    }
}

function escapeHtml(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
}

function escapeAttr(s) {
    return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
