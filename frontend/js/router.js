import { state } from './state.js';
import { loadConfigIntoForm } from './settings.js';
import { loadProjects } from './projects.js';
import { loadLogs } from './log.js';
import { loadTasks } from './tasks.js';

export function navigateTo(page) {
    document.querySelectorAll('.page-content').forEach(el => el.classList.remove('active'));
    const target = document.getElementById('page-' + page);
    if (target) target.classList.add('active');

    document.querySelectorAll('.sidebar .menu-item').forEach(el => el.classList.remove('active'));
    document.querySelectorAll('.sidebar .submenu-item').forEach(el => el.classList.remove('active'));

    const clickedItem = document.querySelector(`.sidebar .menu-item[data-page="${page}"]`);
    if (clickedItem) clickedItem.classList.add('active');

    const clickedSub = document.querySelector(`.sidebar .submenu-item[data-page="chat-${state.currentAgent}"]`);
    if (clickedSub && page === 'chat') clickedSub.classList.add('active');

    state.currentPage = page;

    if (page === 'settings') {
        loadConfigIntoForm();
    }
    if (page === 'projects') {
        loadProjects();
    }
    if (page === 'log') {
        loadLogs();
    }
    if (page === 'taskboard') {
        loadTasks();
    }
}

export function initRouter() {
    document.querySelectorAll('.sidebar .menu-item[data-page]').forEach(btn => {
        btn.addEventListener('click', () => {
            const page = btn.dataset.page;
            if (page === 'chat') {
                const group = btn.closest('.menu-item-group');
                if (group) group.classList.toggle('expanded');
            }
            navigateTo(page);
        });
    });
}
