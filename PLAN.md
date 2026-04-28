# PillArk Implementation Plan

## Phase 1: Core Skeleton (Done)
- [x] Project structure
- [x] Tauri 2.x setup with React + TypeScript
- [x] Basic window configuration (frameless, transparent, always-on-top)
- [x] Process monitoring foundation (sysinfo crate)
- [x] Port abstraction layer for platform-specific code

## Phase 2: UI & Visual States
- [x] Idle state (compact capsule)
- [x] Running state (extended capsule with energy bar)
- [x] Task list expansion on click
- [ ] Task detail view on sub-item click
- [ ] Success/Failed state animations

## Phase 3: Process Monitoring
- [x] Monitor CLI processes (claude, opencode, etc.)
- [ ] Monitor desktop app processes
- [ ] Detect process state changes (start/stop)
- [ ] Add/remove monitored process types

## Phase 4: System Integration
- [ ] System tray icon with menu
- [ ] Global shortcut support
- [ ] Launch at login option
- [ ] Notifications for task completion

## Phase 5: Polish & Cross-Platform
- [ ] Windows port implementation
- [ ] Linux port implementation
- [ ] Settings/preferences UI
- [ ] Auto-updater

## Phase 6: Advanced Features (Future)
- [ ] Task history logging
- [ ] Per-task timing statistics
- [ ] Browser tab monitoring (web AI tools)
- [ ] Mobile companion app
