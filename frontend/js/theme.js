function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('openco-theme', theme);
    document.querySelectorAll('.toggle-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.themeVal === theme);
    });
}

export function initTheme() {
    const saved = localStorage.getItem('openco-theme') || 'dark';
    applyTheme(saved);
}

export function initThemeToggle() {
    document.querySelectorAll('.toggle-btn').forEach(btn => {
        btn.addEventListener('click', () => applyTheme(btn.dataset.themeVal));
    });
}
