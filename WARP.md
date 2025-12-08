# rDock - Windows Application Dock

## Project Overview
rDock is a lightweight, customizable Windows dock application written in Rust. It provides a macOS-like dock experience with auto-hide functionality, custom icon support, and smooth animations.

## Tech Stack
- **Language**: Rust (2021 edition)
- **GUI**: winit for window management, softbuffer for rendering
- **Windows API**: windows-rs crate for native Windows integration
- **Image handling**: image + ico crates for icon loading
- **Configuration**: TOML with hot-reload support via notify crate
- **System tray**: tray-icon crate

## Project Structure
```
rdock/
├── src/
│   ├── main.rs           # Main application loop and window management
│   ├── app_monitor.rs    # Process monitoring for running state indicators
│   ├── config.rs         # TOML configuration parsing
│   ├── context_menu.rs   # Right-click context menu
│   ├── item_editor.rs    # Dock item editing functionality
│   ├── renderer.rs       # Custom 2D rendering engine
│   ├── tooltip.rs        # Hover tooltips
│   └── window_focus.rs   # Window focus management
├── Cargo.toml            # Dependencies and build config
└── config.toml           # User configuration (dock items, theme, etc.)
```

## Key Features
1. **Auto-hide dock**: Slides in/out with smooth animations
2. **Custom icons**: Supports .ico and .png formats
3. **Running indicators**: Shows which apps are currently running
4. **Hot reload**: Automatically reloads when config.toml changes
5. **System tray integration**: Minimize to tray with quit option
6. **Drag reordering**: Reorder dock items by dragging
7. **Taskbar hiding**: Optionally hide Windows taskbar when active

## Configuration
All user configuration is in `config.toml`:
- Dock appearance (size, spacing, colors, opacity, corner radius)
- Auto-hide behavior and delay
- Dock items (applications) with paths, icons, and launch args

## Development Guidelines

### Code Style
- Follow Rust idiomatic patterns and conventions
- Use `anyhow::Result` for error handling
- Keep modules focused and single-purpose
- Add logging with the `log` crate for debugging

### Building
```bash
# Development build
cargo build

# Optimized release build
cargo build --release

# Run in development
cargo run
```

### Testing
```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Windows-Specific Notes
- Uses `#![windows_subsystem = "windows"]` to hide console window
- Requires Windows-specific APIs for taskbar manipulation
- Icon paths in config.toml must use Windows path format (double backslashes or raw strings)

### Performance Optimizations
- Release profile uses LTO, codegen-units=1, and stripping for minimal binary size
- Icon scales update at 60fps (16ms frame time) for smooth animations
- Process checking throttled to every 2 seconds to reduce CPU usage

## Common Tasks

### Adding a new dock item
1. Edit `config.toml`
2. Add `[[items]]` section with name, path, and icon
3. Hot reload triggers automatically (or restart app)

### Changing dock appearance
Modify `[dock]` section in `config.toml`:
- `icon_size`: Icon dimensions in pixels
- `spacing`: Gap between icons
- `padding`: Internal dock padding
- `background_color`: Hex color code
- `background_opacity`: 0.0 to 1.0
- `corner_radius`: Rounded corner radius

### Debugging
- Set `RUST_LOG=debug` or `RUST_LOG=trace` for verbose output
- Use `env_logger` output to diagnose issues
- Check Windows Event Viewer for crash logs

## Known Limitations
- Windows-only (uses Win32 APIs)
- Requires custom icon files (doesn't auto-extract from executables yet)
- Single monitor support (multi-monitor positioning not implemented)

## Dependencies to Watch
- `winit`: Breaking changes in major versions
- `windows`: Large crate, be selective with feature flags
- `notify`: File watching can be platform-specific

## Future Enhancements
- Multi-monitor support
- Auto-extract icons from executables
- Themes and presets
- Plugin system for custom dock items
- Drag-and-drop from desktop to dock
