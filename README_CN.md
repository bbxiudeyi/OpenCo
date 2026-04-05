# OpenCo

AI Agent 协作平台 —— 让多个 AI Agent 像团队一样协同工作。

## 功能

- **Agent 管理** — 创建、配置 AI Agent，分配模型和工具
- **组织架构** — 树形职位结构，Agent 自动获得上下文，支持任务委派
- **实时聊天** — SSE 流式对话，支持工具调用可视化
- **工具系统** — 网页搜索、文件读写、命令执行，带沙箱隔离
- **任务看板** — 创建、分配、跟踪任务进度
- **工作区** — 项目管理，Agent 访问权限控制
- **活动日志** — 记录 Agent 的文件操作和命令执行

## 安装

```bash
npm install -g openco
```

## 使用

```bash
openco              # 启动服务，自动打开浏览器（默认 http://localhost:8181）
openco --port 3000  # 自定义端口
openco --no-browser # 不自动打开浏览器
openco fix          # 修复数据文件 + 检查安装完整性
```

首次启动会在 `~/.openco/` 创建数据目录。

## 支持的模型

通过设置页面配置，支持所有 OpenAI 兼容 API：

- DeepSeek
- GLM（智谱）
- MiniMax
- Kimi（月之暗面）
- Qwen（通义千问）
- 其他兼容 API

## 项目结构

```
OpenCo/
├── backend/           # Rust 后端（axum）
│   └── src/
│       ├── main.rs    # CLI 入口 & HTTP 服务
│       ├── handlers.rs
│       ├── agents.rs
│       ├── config.rs
│       ├── fix.rs     # openco fix 修复逻辑
│       ├── org.rs
│       ├── tasks.rs
│       ├── tools/     # Agent 工具
│       └── ...
├── frontend/          # 前端（HTML + JS + CSS）
├── cli.js             # npm 入口脚本
├── package.json
└── .github/workflows/ # CI 多平台编译
```

## 开发

需要 [Rust](https://rustup.rs/) 和 [Node.js](https://nodejs.org/)。

```bash
# 编译后端
cd backend && cargo build

# 运行（开发模式，数据在项目目录）
cargo run

# 编译 release
cargo build --release
```

## License

MIT
