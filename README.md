# rDock

A lightweight, customizable Windows dock application written in Rust that brings a macOS-like dock experience to Windows.

![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat&logo=windows&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## ✨ Features

- **Auto-hide Dock** - Slides in/out smoothly with configurable delays
- **Custom Icons** - Support for `.ico` and `.png` icon formats
- **Running Indicators** - Visual indicators show which apps are currently running
- **Hot Reload** - Automatically reloads when configuration changes
- **System Tray Integration** - Minimize to tray with quick access
- **Drag Reordering** - Rearrange dock items by dragging
- **Highly Customizable** - Configure appearance, behavior, and applications via TOML

## 📋 Requirements

- Windows 10 or later
- Rust toolchain (for building from source)

## 🚀 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/Randallsm83/rdock.git
cd rdock

# Build release version
cargo build --release

# Run the application
target/release/rdock.exe
```

## ⚙️ Configuration

Configuration is managed through `config.toml`. Place this file in the same directory as the executable.

### Dock Appearance

```toml
[dock]
icon_size = 48              # Icon size in pixels
spacing = 10                # Space between icons
padding = 14                # Internal dock padding
background_color = "#1e1e2e"
background_opacity = 0.92   # 0.0 to 1.0
indicator_color = "#f38ba8" # Running indicator color
corner_radius = 14          # Rounded corners
auto_hide = true            # Enable auto-hide
auto_hide_delay_ms = 400    # Show/hide delay
```

### Adding Applications

```toml
[[items]]
name = "File Explorer"
path = "C:\\Windows\\explorer.exe"
icon = "path\\to\\custom\\icon.ico"

[[items]]
name = "Application with Arguments"
path = "C:\\Path\\To\\App.exe"
args = ["--arg1", "--arg2"]  # Optional launch arguments
icon = "path\\to\\icon.ico"
```

## 🎯 Usage

1. **Launch**: Run `rdock.exe` to start the dock
2. **Show/Hide**: Move your mouse to the bottom of the screen to reveal the dock
3. **Launch Apps**: Click on any icon to launch the application
4. **Reorder**: Drag icons to rearrange them (config auto-updates)
5. **System Tray**: Right-click the tray icon to quit

## 🏗️ Project Structure

```
rdock/
├── src/
│   ├── main.rs           # Application entry and window management
│   ├── app_monitor.rs    # Process monitoring for running indicators
│   ├── config.rs         # TOML configuration parsing
│   ├── context_menu.rs   # Right-click context menu
│   ├── item_editor.rs    # Dock item editing
│   ├── renderer.rs       # 2D rendering engine
│   ├── tooltip.rs        # Hover tooltips
│   └── window_focus.rs   # Window focus management
├── Cargo.toml            # Rust dependencies
└── config.toml           # User configuration
```

## 🔧 Development

```bash
# Development build
cargo build

# Run with debug logging
$env:RUST_LOG="debug"
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## 🎨 Customization Tips

- Use transparent PNG icons for best results
- Keep icon sizes consistent for a polished look
- Adjust `auto_hide_delay_ms` to your preference (lower = more responsive)
- Experiment with `background_opacity` for different visual styles
- Color values support hex format (`#RRGGBB`)

## 📝 License

MIT License - see LICENSE file for details

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 🙏 Acknowledgments

Built with:
- [winit](https://github.com/rust-windowing/winit) - Window management
- [softbuffer](https://github.com/rust-windowing/softbuffer) - Software rendering
- [windows-rs](https://github.com/microsoft/windows-rs) - Windows API bindings
- [tray-icon](https://github.com/tauri-apps/tray-icon) - System tray integration
