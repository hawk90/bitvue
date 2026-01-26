# DEPRECATED - egui UI

**Date**: 2026-01-19
**Status**: Deprecated in favor of Tauri + React

## Migration Notice

This egui-based UI implementation has been deprecated and replaced with a Tauri + React application for better UI/UX and faster development.

## Reason for Deprecation

1. **Limited UI capabilities**: Complex charts (Frame Sizes, B-Pyramid) difficult to implement in egui
2. **Development speed**: React ecosystem provides rich UI libraries (D3.js, Plotly, Ant Design)
3. **VQAnalyzer parity**: Web technologies better suited for replicating commercial analysis tool UI
4. **Modern UX**: React offers better state management, hot reload, and developer tools

## Preserved for Reference

This code remains in the repository as:
- **Reference implementation**: Shows how core Rust types were used
- **Backup**: Can revert if Tauri migration fails
- **Learning resource**: egui patterns and examples

## Key Files

- `src/panels/filmstrip/` - Filmstrip panel with Frame Sizes, B-Pyramid views
- `src/panels/yuv_viewer.rs` - YUV frame viewer with overlays
- `src/workspaces/` - Workspace implementations (Player, Compare, Diagnostics)

## Migration Status

| Component | egui Status | Tauri Status |
|-----------|-------------|--------------|
| File Open | ✅ Complete | 🔄 Migrating |
| Stream Tree | ✅ Complete | 🔄 Migrating |
| Syntax Detail | ✅ Complete | 🔄 Migrating |
| Filmstrip | ✅ Complete | 🔄 Migrating |
| Player | ✅ Complete | 🔄 Migrating |
| Frame Sizes | ⚠️ Partial | 🔄 Migrating |
| B-Pyramid | ⚠️ Partial | 🔄 Migrating |

## Do Not Use for New Features

❌ Do not add new features to this crate
❌ Do not fix non-critical bugs
✅ Critical security/crash bugs only

## New Tauri App

The new application is located at:
```
/Users/hawk/Workspaces/bitvue-tauri/
```

See `TAURI_MIGRATION_PLAN.md` for details.

---

**Maintainers**: This code is kept for historical reference only.
