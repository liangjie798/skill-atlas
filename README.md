# Skill Atlas

Skill Atlas 是一个本地优先、只读扫描的 Agent Skill 资产盘点工具。它将同一 Skill 的逻辑资产与 Codex、Claude Code、Cursor、Gemini CLI 以及共享目录中的安装实例分开建模。

扫描过程会显示实时进度、当前目录计数与可取消状态，完成或失败均使用中文提示。AI 设置内置 OpenAI、DeepSeek、硅基流动、Moonshot、OpenRouter、Ollama 与自定义 OpenAI-compatible 服务商，可以从 `/models` 接口读取可用模型；除了经确认发送 `SKILL.md` 进行分类，也可以让支持联网搜索的模型发现公开 Skill，并保留每条结果的可点击来源。API Key 按服务商分别保存在系统凭据库。

v0.2.1 修复了相同 Skill 安装产生重复健康检查 ID 时的扫描失败问题。健康记录在生成 ID 时包含路径，并在逻辑资产合并后再次去重。

## 下载

访问 [Skill Atlas 官网](https://liangjie798.github.io/skill-atlas/) 或 [GitHub Releases](https://github.com/liangjie798/skill-atlas/releases)。官网提供最新 Windows 安装包与 SHA-256 校验入口。首版安装包尚未进行 Windows 代码签名，首次运行可能出现 SmartScreen 提示。

## 仓库结构

- `apps/desktop`：Tauri 2、React、TypeScript 与 Rust 桌面应用。
- `apps/website`：React/Vite 静态官网，通过 GitHub Pages 发布。
- `packages/contracts`：前端共享的领域类型与分类契约。

## 本地开发

```powershell
pnpm install
pnpm dev
```

## 验证与打包

```powershell
pnpm lint
pnpm test
pnpm build
pnpm tauri:build
```

构建链只使用 Node.js/pnpm、Rust/Cargo 与 Tauri，不依赖 Python。
