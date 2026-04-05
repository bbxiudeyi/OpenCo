export function initInteraction() {
    const btns = document.querySelectorAll('.interaction-btn');
    const panels = document.querySelectorAll('.interaction-panel');

    function showPanel(name) {
        panels.forEach(p => p.style.display = 'none');
        const target = document.getElementById('panel-' + name);
        if (target) target.style.display = 'flex';
        btns.forEach(b => b.classList.toggle('active', b.dataset.panel === name));
    }

    btns.forEach(b => {
        b.addEventListener('click', () => showPanel(b.dataset.panel));
    });

    // Sign In submit — placeholder
    document.getElementById('signin-submit')?.addEventListener('click', () => {
        const email = document.getElementById('signin-email')?.value?.trim();
        const password = document.getElementById('signin-password')?.value;
        if (!email || !password) {
            alert('Please fill in email and password.');
            return;
        }
        alert('Sign in is not yet connected to a backend. Coming soon!');
    });

    // Sign Up submit — placeholder
    document.getElementById('signup-submit')?.addEventListener('click', () => {
        const email = document.getElementById('signup-email')?.value?.trim();
        const password = document.getElementById('signup-password')?.value;
        const confirm = document.getElementById('signup-confirm')?.value;
        if (!email || !password || !confirm) {
            alert('Please fill in all fields.');
            return;
        }
        if (password !== confirm) {
            alert('Passwords do not match.');
            return;
        }
        alert('Sign up is not yet connected to a backend. Coming soon!');
    });
}
