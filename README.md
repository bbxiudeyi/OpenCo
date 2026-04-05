# OpenCo

AI Agent collaboration platform — multiple AI agents working together as a team.

## Features

- **Agent Management** — Create and configure AI agents with custom models and tools
- **Org Chart** — Tree-structured positions with automatic context, supports task delegation
- **Live Chat** — SSE streaming with tool call visualization
- **Tool System** — Web search, file read/write, command execution with sandbox isolation
- **Task Board** — Create, assign, and track task progress
- **Workspace** — Project management with agent access control
- **Activity Log** — Track agent file operations and command executions

## Install

```bash
npm install -g openco
```

## Usage

```bash
openco              # Start server, opens browser automatically (default: http://localhost:8181)
openco --port 3000  # Custom port
openco --no-browser # Don't open browser
openco fix          # Repair data files + check installation integrity
```

Data is stored in `~/.openco/` on first run.

## Supported Models

Configure via the settings page. Supports all OpenAI-compatible APIs:

- DeepSeek
- GLM (Zhipu)
- MiniMax
- Kimi (Moonshot)
- Qwen (Tongyi)
- Other compatible APIs

## Project Structure

```
OpenCo/
├── backend/           # Rust backend (axum)
│   └── src/
│       ├── main.rs    # CLI entry & HTTP server
│       ├── handlers.rs
│       ├── agents.rs
│       ├── config.rs
│       ├── fix.rs     # openco fix repair logic
│       ├── org.rs
│       ├── tasks.rs
│       ├── tools/     # Agent tools
│       └── ...
├── frontend/          # Frontend (HTML + JS + CSS)
├── cli.js             # npm entry script
├── package.json
└── README_CN.md       # 中文文档
```

## Development

Requires [Rust](https://rustup.rs/) and [Node.js](https://nodejs.org/).

```bash
# Build backend
cd backend && cargo build

# Run (dev mode, data in project directory)
cargo run

# Build release
cargo build --release
```

## License

MIT
