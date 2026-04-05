import { state, chatEl, inputEl, sendBtnEl, agentListEl } from './state.js';
import { loadHistoryFromBackend, appendMsg, updateProgress } from './chat.js';
import { navigateTo } from './router.js';

export async function loadAgents() {
    try {
        const res = await fetch('/api/agents');
        const data = await res.json();
        state.agents = data.agents || [];
        renderAgentList();

        if (!state.currentAgent && state.agents.length > 0) {
            selectAgent(state.agents[0].name);
        }
    } catch (e) {
        console.error('Failed to load agents:', e);
    }
}

// Build agent name → position title map from org data
function getAgentTitleMap() {
    const map = {};
    (state.orgData?.positions || []).forEach(pos => {
        (pos.agents || []).forEach(name => {
            map[name] = pos.title;
        });
    });
    return map;
}

let _searchComposing = false;

function ensureSearchBox() {
    if (!agentListEl) return null;
    let searchInput = agentListEl.querySelector('.agent-search-input');
    if (searchInput) return searchInput;

    agentListEl.innerHTML = '';

    const searchWrap = document.createElement('div');
    searchWrap.className = 'agent-search-wrap';
    searchInput = document.createElement('input');
    searchInput.type = 'text';
    searchInput.className = 'agent-search-input';
    searchInput.placeholder = 'Search agent...';
    searchInput.addEventListener('compositionstart', () => { _searchComposing = true; });
    searchInput.addEventListener('compositionend', () => {
        _searchComposing = false;
        filterAgentButtons(searchInput.value);
    });
    searchInput.addEventListener('input', () => {
        if (!_searchComposing) filterAgentButtons(searchInput.value);
    });
    searchWrap.appendChild(searchInput);
    agentListEl.appendChild(searchWrap);

    // Placeholder for agent buttons
    const listDiv = document.createElement('div');
    listDiv.className = 'agent-list-items';
    agentListEl.appendChild(listDiv);

    return searchInput;
}

function filterAgentButtons(filter = '') {
    const listDiv = agentListEl?.querySelector('.agent-list-items');
    if (!listDiv) return;
    listDiv.innerHTML = '';

    const titleMap = getAgentTitleMap();
    const keyword = filter.trim().toLowerCase();

    const filtered = keyword
        ? state.agents.filter(a => {
            const display = titleMap[a.name] ? `${a.name}(${titleMap[a.name]})` : a.name;
            return display.toLowerCase().includes(keyword) || a.name.toLowerCase().includes(keyword);
        })
        : state.agents;

    filtered.forEach(agent => {
        const btn = document.createElement('button');
        btn.className = 'submenu-item' + (state.currentAgent === agent.name ? ' active' : '');
        btn.dataset.agent = agent.name;
        btn.dataset.page = 'chat-' + agent.name;
        const title = titleMap[agent.name];
        const label = title ? `${agent.name}(${title})` : agent.name;
        const dot = document.createElement('span');
        dot.className = 'dot dot-purple';
        const labelSpan = document.createElement('span');
        labelSpan.textContent = label;
        btn.appendChild(dot);
        btn.appendChild(labelSpan);
        btn.addEventListener('click', () => {
            selectAgent(agent.name);
            navigateTo('chat');
        });
        listDiv.appendChild(btn);
    });
}

export function renderAgentList(filter = '') {
    const searchInput = ensureSearchBox();
    if (searchInput && filter !== undefined) {
        searchInput.value = filter;
    }
    filterAgentButtons(filter);
}

export function selectAgent(name) {
    state.currentAgent = name;

    document.querySelectorAll('.submenu-item[data-agent]').forEach(el => {
        el.classList.toggle('active', el.dataset.agent === name);
    });

    const chatTitle = document.querySelector('.chat-title');
    if (chatTitle) chatTitle.textContent = name;

    chatEl.innerHTML = '';
    state.agentMessages[name] = [];
    loadHistoryFromBackend(name);

    const busy = state.agentSending[name] || false;
    sendBtnEl.disabled = busy;
    inputEl.disabled = busy;
    if (!busy) inputEl.focus();

    updateProgress();
}
