export function initContactBar() {
    const btn = document.getElementById('contact-copy-btn');
    if (!btn) return;
    btn.addEventListener('click', () => {
        navigator.clipboard.writeText('eulerguy9137@163.com').then(() => {
            btn.classList.add('copied');
            setTimeout(() => btn.classList.remove('copied'), 1500);
        });
    });
}
