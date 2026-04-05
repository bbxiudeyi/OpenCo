// ===== DOM Elements =====
export const chatEl = document.getElementById('chat');
export const inputEl = document.getElementById('input');
export const sendBtnEl = document.getElementById('send-btn');
export const agentListEl = document.getElementById('agent-list');

// ===== Constants =====
export const providerDefaults = {
    deepseek: { url: 'https://api.deepseek.com', model: 'deepseek-chat' },
    kimi: { url: 'https://api.moonshot.cn/v1', model: 'moonshot-v1-8k' },
    qwen: { url: 'https://dashscope.aliyuncs.com/compatible-mode/v1', model: 'qwen-turbo' },
    zhipu: { url: 'https://open.bigmodel.cn/api/paas/v4', model: 'glm-4-flash' },
    minimax: { url: 'https://api.minimax.chat/v1', model: 'MiniMax-Text-01' },
};

// ===== Shared Mutable State =====
export const state = {
    currentPage: 'chat',
    agents: [],                 // [{name, model, tools}]
    currentAgent: null,         // selected agent name
    cachedConfig: null,

    // Per-agent state
    agentMessages: {},
    agentSending: {},           // { agentName: true/false }

    // Org
    orgData: { positions: [] },
    selectedPositionId: null,
    panelHasChanges: false,
    currentZoom: 1,
    panX: 0,
    panY: 0,
};
