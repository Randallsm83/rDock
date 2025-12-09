# rDock Project Rules

## Project Overview
rDock is a lightweight Windows dock application written in Rust, providing a macOS-like dock experience for Windows 10/11.

## Architecture

### Core Components
- **main.rs** - Application entry, window management, event loop, dock state machine
- **renderer.rs** - 2D software rendering with icon magnification and animations
- **config.rs** - TOML configuration parsing and hot-reload
- **tray_popup.rs** - Windows 11 system tray overflow integration

### Key Patterns
- Uses `winit` for window management with custom `ApplicationHandler`
- Software rendering via `softbuffer` (no GPU required)
- Windows API calls via `windows-rs` crate
- System tray via `tray-icon` crate

## Development Guidelines

### Building
```powershell
cargo build --release
```

### Testing Changes
1. Kill existing rdock: `taskkill /F /IM rdock.exe`
2. Build: `cargo build --release`
3. Run: `.\target\release\rdock.exe`

### Windows API Notes
- Windows 11 taskbar is XAML-based - no Win32 child windows exist under TrayNotifyWnd
- Use keyboard shortcuts (Win+B, Enter) for system tray interaction
- Always use `SendInput` for simulating keyboard/mouse, not `PostMessage` for protected windows
- Taskbar hiding requires both `ShowWindow(SW_HIDE)` AND `SetWindowPos` off-screen

### Code Style
- Use `unsafe` blocks sparingly, only for Windows API calls
- Prefer `if let Ok(x) = ...` over `.unwrap()` for Windows API results
- All user-facing strings should be in config, not hardcoded

## Special Items
Special items are dock items that trigger built-in Windows functions instead of launching apps.
Defined via `special = "item_type"` in config.toml.

### Adding New Special Items
1. Add case to `launch_special()` in `main.rs`
2. Document in README.md
3. Test on both Windows 10 and 11 if possible

## Configuration
- Config file: `config.toml` in same directory as executable
- Hot-reload: Changes auto-detected and applied
- Icon paths: Absolute paths recommended, supports `.ico` and `.png`
