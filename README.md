# rDock

A lightweight, customizable Windows dock application written in Rust that brings a macOS-like dock experience to Windows.

![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat&logo=windows&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Screenshots

<table>
  <tr>
    <td><img src=".github/images/dock-main.png" alt="Main dock view" width="600"/></td>
  </tr>
  <tr>
    <td align="center"><em>rdock with custom icons and running indicators</em></td>
  </tr>
</table>

<details>
<summary>More Screenshots</summary>

<table>
  <tr>
    <td><img src=".github/images/dock-context-menu.png" alt="Context menu" width="400"/></td>
    <td><img src=".github/images/dock-hover.png" alt="Hover tooltip" width="400"/></td>
  </tr>
  <tr>
    <td align="center"><em>Right-click context menu</em></td>
    <td align="center"><em>Application tooltips</em></td>
  </tr>
</table>

</details>

## ✨ Features

- **Auto-hide Dock** - Slides in/out smoothly with configurable delays
- **Custom Icons** - Support for `.ico` and `.png` icon formats
- **Running Indicators** - Visual indicators show which apps are currently running
- **Hot Reload** - Automatically reloads when configuration changes
- **System Tray Integration** - Minimize to tray with quick access
- **Drag Reordering** - Rearrange dock items by dragging
- **Highly Customizable** - Configure appearance, behavior, and applications via TOML
- **Lightweight & Efficient** - Minimal resource usage (see performance section below)

## 📋 Requirements

- Windows 10 or later
- Rust toolchain (for building from source)

## 🚀 Installation

### Pre-built Binary (Recommended)

1. Download the latest release from [Releases](https://github.com/Randallsm83/rDock/releases)
2. Extract the ZIP file to your preferred location
3. Edit `config.toml` to customize your dock
4. Run `rdock.exe`

### From Source

```bash
# Clone the repository
git clone https://github.com/Randallsm83/rDock.git
cd rDock

# Build release version
cargo build --release

# Run the application
target/release/rdock.exe
```

## ⚙️ Configuration

Configuration can be managed in two ways:
1. **GUI**: Right-click on the dock or icons to access configuration options
2. **Manual**: Edit `config.toml` directly (changes reload automatically)

Place `config.toml` in the same directory as the executable.

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
5. **Context Menu**: Right-click on icons or the dock background for configuration options
6. **System Tray**: Right-click the tray icon to quit

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

## ⚡ Performance

rdock is built for efficiency. Compared to similar dock applications:

- **~75% less memory** - Uses only ~7 MB of private memory vs ~27 MB typical
- **~84% smaller binary** - Just 2.9 MB vs 18+ MB for comparable applications
- **Minimal system impact** - Only 3 threads and 207 handles vs 8+ threads and 700+ handles
- **Fast startup** - Optimized release build with LTO and minimal dependencies

These optimizations mean rdock runs smoothly without impacting your system's performance, even on resource-constrained machines.

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

## 📦 Future Distribution

Planned package manager support:
- **winget**: `winget install rdock` (pending)
- **scoop**: `scoop install rdock` (pending)
- **Chocolatey**: `choco install rdock` (pending)

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
