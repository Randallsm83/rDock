# rdock AGENTS.md

Lightweight, customizable Windows dock — macOS-style. Auto-hide, custom icons, running indicators, hot-reload config, system-tray overflow.

## Stack

Rust 2021. `winit` 0.30 + `softbuffer` for windowing/2D rendering. `windows-rs` 0.58 for Win32 (Shell, GDI, processes, COM). `image` + `ico` for icon loading. `tray-icon` for system tray. `notify` 6.1 for config hot-reload. `serde` + `toml` for config. `log` + `env_logger`.

## Commands

```powershell
cargo build               # debug
cargo build --release     # release; LTO, codegen-units=1, strip=true
cargo run                 # debug run
cargo test
cargo clippy
cargo fmt

$env:RUST_LOG="debug"; cargo run   # verbose runtime logs
```

Release profile is intentionally tight — keep the binary small (~2.9 MB target).

## Layout

```
src/
├── main.rs           entry + window management
├── app_monitor.rs    process polling for running indicators
├── config.rs         TOML parsing
├── context_menu.rs   right-click menu
├── item_editor.rs    in-app dock-item editing
├── renderer.rs       2D rendering
├── tooltip.rs        hover tooltips
├── tray_popup.rs     hidden-tray-icon overflow
└── window_focus.rs   focus management
```

`config.toml` lives next to the executable, hot-reloaded via `notify`.

## Conventions

- **Special-item slugs are stable identifiers** — values like `start_menu`, `system_tray`, `recycle_bin`, `show_desktop`, `task_view`, `quick_settings`, etc. are part of the user-facing config schema. Adding a new special item is fine; **renaming or removing one breaks user configs**.
- **Performance is a feature** — see [PERFORMANCE.md](file:///D:/rdock/PERFORMANCE.md). Target: ~7 MB private memory, 3 threads, ~207 handles. Don't add a tokio runtime or heavy crates without justification.
- **Stability fixes** — see [STABILITY_FIXES.md](file:///D:/rdock/STABILITY_FIXES.md) before changing window-message handling.
- **Releases** — follow [RELEASING.md](file:///D:/rdock/RELEASING.md). GitHub releases are user-facing.
- **Sibling docs**: legacy WARP guidance lives in `WARP.md`; this file supersedes it for AGENTS.md-aware tools.

## Notes

- Windows 10+ only — uses APIs that don't exist on older Windows.
- Firefox uses its own trust store for any TLS interactions and won't pick up system roots; not relevant for this project today, but flagged because some special-item launchers spawn browsers.
- Workspace conventions: [D:\AGENTS.md](file:///D:/AGENTS.md).
