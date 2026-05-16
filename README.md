<a id="english"></a>

# PillArk

> Turn Claude Code background tasks into a quiet, visible desktop pill.

[English](#english) | [中文](#zh-cn)

PillArk is a small macOS desktop utility that floats near the top of your screen and keeps an eye on Claude Code task status in real time. When nothing is running, it stays as a compact pill. When Claude starts working, it switches into a running state. When a task finishes, it expands briefly so you know what just completed.

If you often start a Claude Code task and then move back to your editor, browser, or another window, PillArk gives you a lightweight status island without forcing you to keep checking the terminal.

## Download 📥

[Download PillArk for macOS](https://github.com/Richerlv/pill-ark/releases/latest/download/PillArk_0.1.0_aarch64.dmg)

The link above downloads the latest DMG installer from GitHub Releases. After downloading, open the DMG and drag `PillArk.app` into `Applications`.

## Preview ✨

### Idle 💊

When no active Claude task is detected, PillArk stays in its minimal idle state.

![PillArk idle state](user_case/start.png)

### Running ⚡

When a Claude Code session enters `running`, `busy`, `working`, or `thinking`, the pill updates in real time and shows the number of active tasks.

![PillArk task running state](user_case/task_running.png)

### Completed ✅

When a task finishes, PillArk expands automatically, shows the completed task, and then returns to the idle state after a few seconds.

![PillArk task completed state](user_case/task_completed.png)

## What It Does 🧠

- **Tracks Claude Code in real time**: Reads local Claude session state every second.
- **Floats as a desktop pill**: A lightweight Tauri window that stays visible without getting in the way.
- **Shows clear visual states**: Idle, running, and completion states are easy to distinguish at a glance.
- **Expands on completion**: Completed tasks are surfaced automatically, so you do not need to keep checking the terminal.
- **Shows task details**: Click the pill to expand the task list and inspect task names and PIDs.

## Supported Platforms 🛠️

PillArk currently supports:

- Claude Code (`claude`)
- macOS

It detects task status by reading `~/.claude/sessions/*.json`. A session is treated as active when its status is `running`, `busy`, `working`, or `thinking`.

## Quick Start 🚀

### Requirements

- Node.js 18+
- Rust 1.70+
- macOS

### Install Dependencies

```bash
npm install
```

### Run in Development

```bash
npm run dev
```

This starts the Tauri development server and opens the PillArk desktop window.

### Build the App

```bash
npm run build
```

Tauri will generate the desktop build output for local packaging.

## Usage 🧭

1. Start PillArk.
2. Open Claude Code and start a task.
3. PillArk switches to the running state within about one second.
4. When the task finishes, PillArk expands with a completion notice.
5. After a few seconds, the pill collapses back to idle.

## Project Structure 🧩

```text
pill-ark/
├── src/                    # React frontend
│   ├── App.tsx             # Main UI and state transitions
│   └── styles.css          # Pill window styling
├── src-tauri/              # Tauri / Rust backend
│   ├── src/
│   │   ├── main.rs         # App entry point
│   │   ├── lib.rs          # Tauri setup
│   │   ├── core/           # Claude session reading and task detection
│   │   └── ports/          # Platform-specific capabilities
│   └── tauri.conf.json     # Tauri configuration
├── user_case/              # README screenshots
└── package.json
```

## Commands 📦

```bash
npm run dev        # Start the desktop app in development
npm run dev:web    # Start only the Vite frontend
npm run build:web  # Build the frontend
npm run build      # Build the Tauri app
```

## Roadmap 🗺️

- More AI agent / CLI integrations
- System tray menu
- Task completion notifications
- Task history
- Settings panel
- Windows / Linux support

## License 📄

MIT

---

<a id="zh-cn"></a>

# 中文

> 把 Claude Code 的后台任务，变成一颗安静但醒目的桌面胶囊。

[English](#english) | [中文](#zh-cn)

PillArk 是一个 macOS 桌面小工具：它会悬浮在屏幕上方，实时监听 Claude Code 任务状态。当没有任务运行时，它会保持紧凑的胶囊形态；当 Claude 开始工作时，它会切换成运行态；当任务结束时，它会自动展开，告诉你刚刚完成了什么。

如果你经常一边开着 Claude Code 跑任务，一边切到浏览器、编辑器或其他窗口里做事，PillArk 可以给你一个轻量的状态岛，不用反复回到终端确认进度。

## 下载 📥

[下载 macOS 版 PillArk](https://github.com/Richerlv/pill-ark/releases/latest/download/PillArk_0.1.0_aarch64.dmg)

上面的链接会从 GitHub Releases 下载最新的 DMG 安装包。下载后打开 DMG，把 `PillArk.app` 拖到 `Applications` 即可安装。

## 效果预览 ✨

### 空闲待命 💊

没有检测到正在运行的 Claude 任务时，PillArk 会保持最小胶囊形态。

![PillArk idle state](user_case/start.png)

### 任务运行中 ⚡

当 Claude Code session 进入 `running`、`busy`、`working` 或 `thinking` 状态时，胶囊会实时变成运行态，并显示当前任务数量。

![PillArk task running state](user_case/task_running.png)

### 完成提示 ✅

任务结束后，PillArk 会自动展开，短暂展示完成状态和任务信息，然后再收回到空闲状态。

![PillArk task completed state](user_case/task_completed.png)

## 它能做什么 🧠

- **实时监听 Claude Code**：每秒读取本机 Claude session 状态，自动发现正在运行的任务。
- **悬浮胶囊窗口**：基于 Tauri 的轻量桌面窗口，保持可见但不打扰。
- **三种视觉状态**：空闲、运行中、完成提示，状态变化一眼能看懂。
- **任务完成自动展开**：不用反复切回终端确认，完成时会主动提醒你。
- **点击查看任务列表**：点击胶囊即可展开任务列表，查看任务名称和 PID。

## 当前支持 🛠️

PillArk 当前支持：

- Claude Code (`claude`)
- macOS

它通过读取 `~/.claude/sessions/*.json` 判断 Claude session 状态。只要 session 状态是 `running`、`busy`、`working` 或 `thinking`，就会被视为正在运行的任务。

## 快速开始 🚀

### 环境要求

- Node.js 18+
- Rust 1.70+
- macOS

### 安装依赖

```bash
npm install
```

### 开发模式启动

```bash
npm run dev
```

这会启动 Tauri 开发服务，并打开 PillArk 桌面窗口。

### 构建应用

```bash
npm run build
```

Tauri 会生成桌面应用构建产物，便于后续本地打包。

## 使用方式 🧭

1. 启动 PillArk。
2. 打开 Claude Code，并开始一个任务。
3. PillArk 会在约 1 秒内切换到运行态。
4. 任务结束后，PillArk 自动展开完成提示。
5. 提示停留数秒后自动收起。

## 项目结构 🧩

```text
pill-ark/
├── src/                    # React 前端
│   ├── App.tsx             # 主 UI 和状态切换逻辑
│   └── styles.css          # 胶囊窗口样式
├── src-tauri/              # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs         # 应用入口
│   │   ├── lib.rs          # Tauri 初始化
│   │   ├── core/           # Claude session 读取与任务判断
│   │   └── ports/          # 平台相关能力
│   └── tauri.conf.json     # Tauri 配置
├── user_case/              # README 效果图
└── package.json
```

## 常用命令 📦

```bash
npm run dev        # 启动桌面开发版
npm run dev:web    # 只启动 Vite 前端
npm run build:web  # 构建前端
npm run build      # 构建 Tauri 应用
```

## Roadmap 🗺️

- 支持更多 AI agent / CLI 工具
- 系统托盘菜单
- 任务完成通知
- 任务历史记录
- 设置面板
- Windows / Linux 支持

## License 📄

MIT
