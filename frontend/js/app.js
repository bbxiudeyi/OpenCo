import { initTheme, initThemeToggle } from './theme.js';
import { initRouter } from './router.js';
import { initChat } from './chat.js';
import { loadAgents } from './agents.js';
import { initSettings, loadConfigIntoForm } from './settings.js';
import { loadOrg, initOrg } from './org.js';
import { initInteraction } from './interaction.js';
import { initContactBar } from './contact-bar.js';
import { initProjects } from './projects.js';
import { initLog, loadLogs } from './log.js';
import { initTasks, loadTasks } from './tasks.js';

document.addEventListener('DOMContentLoaded', () => {
    if (typeof lucide !== 'undefined') {
        lucide.createIcons();
    }
    initTheme();
    initThemeToggle();
    initRouter();
    initChat();
    initSettings();
    initInteraction();
    initContactBar();
    initProjects();
    initLog();
    initTasks();
    loadAgents().then(() => loadOrg());
    initOrg();
});
