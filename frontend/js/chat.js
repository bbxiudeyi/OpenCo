import { state, chatEl, inputEl, sendBtnEl } from './state.js';

export async function getConfig() {
    if (state.cachedConfig) return state.cachedConfig;
    const res = await fetch('/api/config');
    state.cachedConfig = await res.json();
    return state.cachedConfig;
}

function getTimeStr() {
    const now = new Date();
    return now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0');
}

export function appendMsg(role, text) {
    const row = document.createElement('div');
    row.className = 'msg-row ' + role;

    if (role === 'assistant') {
        const avatar = document.createElement('div');
        avatar.className = 'msg-avatar';
        avatar.innerHTML = '<i data-lucide="bot"></i>';

        const wrap = document.createElement('div');
        wrap.className = 'msg-wrap';

        const bubble = document.createElement('div');
        bubble.className = 'msg-bubble';
        bubble.textContent = text;

        const time = document.createElement('div');
        time.className = 'msg-time';
        time.textContent = getTimeStr();

        wrap.appendChild(bubble);
        wrap.appendChild(time);
        row.appendChild(avatar);
        row.appendChild(wrap);
    } else {
        const wrap = document.createElement('div');
        wrap.className = 'msg-wrap';

        const bubble = document.createElement('div');
        bubble.className = 'msg-bubble';
        bubble.textContent = text;

        const time = document.createElement('div');
        time.className = 'msg-time';
        time.textContent = getTimeStr();

        wrap.appendChild(bubble);
        wrap.appendChild(time);
        row.appendChild(wrap);
    }

    chatEl.appendChild(row);
    chatEl.scrollTop = chatEl.scrollHeight;

    if (typeof lucide !== 'undefined') {
        lucide.createIcons({ nodes: [row] });
    }

    return row;
}

export async function loadHistoryFromBackend(name) {
    try {
        const res = await fetch(`/api/agents/${encodeURIComponent(name)}/history`);
        const data = await res.json();
        const entries = data.messages || [];
        state.agentMessages[name] = entries.map(e => ({ role: e.role, text: e.content }));
        if (state.currentAgent === name) {
            chatEl.innerHTML = '';
            state.agentMessages[name].forEach(m => appendMsg(m.role, m.text));
        }
    } catch (e) {
        console.error('Failed to load history:', e);
    }
}

export function updateProgress() {
    const progressText = document.querySelector('.progress-text');
    const progressFill = document.querySelector('.progress-fill');
    if (!progressText || !progressFill) return;

    const agent = state.agents.find(a => a.name === state.currentAgent);
    if (agent) {
        progressText.textContent = agent.tools.length + ' tools';
        const pct = Math.min(100, agent.tools.length * 25);
        progressFill.style.width = pct + '%';
    }
}

function createStepEl(step) {
    const el = document.createElement('div');
    el.className = 'tool-step';
    el.dataset.name = step.name;
    el.dataset.status = step.status || 'done';

    const nameEl = document.createElement('span');
    nameEl.className = 'tool-step-name';
    nameEl.textContent = step.name;

    const detailEl = document.createElement('span');
    detailEl.className = 'tool-step-detail';
    try {
        const args = JSON.parse(step.input);
        const summary = args.query || args.path || args.command || step.input;
        detailEl.textContent = summary.length > 60 ? summary.slice(0, 60) + '...' : summary;
    } catch {
        detailEl.textContent = step.input.length > 60 ? step.input.slice(0, 60) + '...' : step.input;
    }

    el.appendChild(nameEl);
    el.appendChild(detailEl);
    return el;
}

async function sendMessage() {
    if (state.agentSending[state.currentAgent]) return;
    if (!state.currentAgent) {
        alert('Please select or create an agent first.');
        return;
    }

    const text = inputEl.value.trim();
    if (!text) return;

    state.agentSending[state.currentAgent] = true;
    const thisAgent = state.currentAgent;
    sendBtnEl.disabled = true;
    inputEl.disabled = true;
    inputEl.value = '';

    appendMsg('user', text);

    if (!state.agentMessages[state.currentAgent]) state.agentMessages[state.currentAgent] = [];
    state.agentMessages[state.currentAgent].push({ role: 'user', text });

    const messages = state.agentMessages[state.currentAgent]
        .map(m => ({ role: m.role, content: m.text }));

    const loadingEntry = { role: 'assistant', text: '...', loading: true };
    state.agentMessages[state.currentAgent].push(loadingEntry);

    const loadingRow = appendMsg('assistant', '...');
    loadingRow.dataset.loading = 'true';

    function findLoadingBubble() {
        const row = chatEl.querySelector('.msg-row[data-loading="true"]');
        if (!row) return { row: null, bubble: null, wrap: null };
        return {
            row,
            bubble: row.querySelector('.msg-bubble'),
            wrap: row.querySelector('.msg-wrap'),
        };
    }

    let stepsEl = null;

    try {
        const res = await fetch(`/api/agents/${encodeURIComponent(state.currentAgent)}/chat`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ messages }),
        });

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });

            let boundary;
            while ((boundary = buffer.indexOf('\n\n')) !== -1) {
                const raw = buffer.substring(0, boundary);
                buffer = buffer.substring(boundary + 2);

                let eventType = '';
                let data = '';
                for (const line of raw.split('\n')) {
                    if (line.startsWith('event:')) eventType = line.substring(6).trim();
                    else if (line.startsWith('data:')) data = line.substring(5).trim();
                }

                if (!data) continue;

                let parsed;
                try { parsed = JSON.parse(data); } catch { continue; }

                if (eventType === 'step') {
                    if (state.currentAgent === thisAgent) {
                        const dom = findLoadingBubble();
                        if (dom.wrap && dom.bubble) {
                            if (!stepsEl) {
                                stepsEl = document.createElement('div');
                                stepsEl.className = 'tool-steps';
                                dom.wrap.insertBefore(stepsEl, dom.bubble);
                            }

                            if (parsed.status === 'running') {
                                const stepEl = createStepEl(parsed);
                                stepsEl.appendChild(stepEl);
                            } else if (parsed.status === 'done') {
                                const stepEl = stepsEl.querySelector(
                                    `.tool-step[data-name="${parsed.name}"][data-status="running"]`
                                );
                                if (stepEl) {
                                    stepEl.dataset.status = 'done';
                                } else {
                                    const el = createStepEl(parsed);
                                    stepsEl.appendChild(el);
                                }
                            }

                            chatEl.scrollTop = chatEl.scrollHeight;
                        }
                    }
                } else if (eventType === 'done') {
                    loadingEntry.text = parsed.reply || '(empty)';
                    loadingEntry.loading = false;
                    if (state.currentAgent === thisAgent) {
                        const dom = findLoadingBubble();
                        if (dom.bubble) dom.bubble.textContent = loadingEntry.text;
                        if (dom.row) delete dom.row.dataset.loading;
                    }
                } else if (eventType === 'error') {
                    loadingEntry.text = 'Error: ' + (parsed.error || 'Unknown error');
                    loadingEntry.loading = false;
                    if (state.currentAgent === thisAgent) {
                        const dom = findLoadingBubble();
                        if (dom.bubble) dom.bubble.textContent = loadingEntry.text;
                        if (dom.row) delete dom.row.dataset.loading;
                    }
                }
            }
        }
    } catch (e) {
        loadingEntry.text = 'Error: ' + e.message;
        loadingEntry.loading = false;
        if (state.currentAgent === thisAgent) {
            const dom = findLoadingBubble();
            if (dom.bubble) dom.bubble.textContent = loadingEntry.text;
            if (dom.row) delete dom.row.dataset.loading;
        }
    }

    chatEl.scrollTop = chatEl.scrollHeight;
    state.agentSending[thisAgent] = false;
    if (state.currentAgent === thisAgent) {
        sendBtnEl.disabled = false;
        inputEl.disabled = false;
        inputEl.focus();
    }
}

export function initChat() {
    sendBtnEl.addEventListener('click', sendMessage);
    inputEl.addEventListener('keydown', e => {
        if (e.key === 'Enter') sendMessage();
    });
}
