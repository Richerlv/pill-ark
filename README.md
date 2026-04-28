# PillArk

> A system tray app for tracking AI agent tasks on macOS

## Features

- **Static Pill**: Minimalist capsule that stays on top of your screen
- **Process Monitoring**: Tracks AI tools (Claude Code, Cursor, Trae, OpenCode, etc.)
- **Visual States**: Idle, Running (with energy animation), and expanded task list
- **Cross-Platform Ready**: Architecture designed for Windows support

## Prerequisites

- Node.js 18+
- Rust 1.70+
- macOS (currently supported)

## Setup

```bash
# Install dependencies
npm install

# Build Rust backend
cd src-tauri && cargo build
cd ..
```

## Run

```bash
npm run dev
```

This will start the Tauri dev server and launch the PillArk window.

## Architecture

```
pill-ark/
├── src/                    # React frontend
│   ├── App.tsx            # Main UI component
│   └── styles.css         # Styling
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── lib.rs         # Tauri setup
│   │   ├── core/          # Business logic
│   │   └── ports/         # Platform abstraction
│   └── tauri.conf.json    # Tauri config
└── package.json
```

## Supported AI Tools

- Claude Code (`claude`)
- Cursor (`cursor`)
- Trae (`trae`)
- Windsurf (`windsurf`)
- Warp (`warp`)
- OpenCode (`opencode`)
- VS Code (`code`)

## License

MIT
