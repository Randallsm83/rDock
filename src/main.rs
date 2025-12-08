#![windows_subsystem = "windows"]

mod app_monitor;
mod config;
mod context_menu;
mod item_editor;
mod renderer;
mod tooltip;
mod window_focus;

use anyhow::Result;
use config::{Config, DockItem};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use renderer::Renderer;
use tooltip::Tooltip;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::windows::{WindowAttributesExtWindows, WindowExtWindows};
use winit::window::{Window, WindowId, WindowLevel};

const PROCESS_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const ANIMATION_FRAME_TIME: Duration = Duration::from_millis(16);
const HIDE_DELAY: Duration = Duration::from_millis(500);


/// Hide or show the Windows taskbar
#[cfg(windows)]
fn set_taskbar_visibility(visible: bool) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::PCWSTR;
    
    unsafe {
        let class_name: Vec<u16> = "Shell_TrayWnd".encode_utf16().chain(std::iter::once(0)).collect();
        if let Ok(taskbar) = FindWindowW(PCWSTR(class_name.as_ptr()), PCWSTR::null()) {
            if !taskbar.0.is_null() {
                let cmd = if visible { SW_SHOW } else { SW_HIDE };
                let _ = ShowWindow(taskbar, cmd);
            }
        }
        
        // Also handle the secondary taskbar on multi-monitor setups
        let class_name2: Vec<u16> = "Shell_SecondaryTrayWnd".encode_utf16().chain(std::iter::once(0)).collect();
        if let Ok(taskbar2) = FindWindowW(PCWSTR(class_name2.as_ptr()), PCWSTR::null()) {
            if !taskbar2.0.is_null() {
                let cmd = if visible { SW_SHOW } else { SW_HIDE };
                let _ = ShowWindow(taskbar2, cmd);
            }
        }
    }
}

/// Create a tray icon with a dock-like design (3 rounded squares)
fn create_tray_icon(color_hex: &str) -> Result<tray_icon::Icon, tray_icon::BadIcon> {
    const SIZE: usize = 32;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    
    // Parse color
    let hex = color_hex.trim_start_matches('#');
    let (r, g, b) = if hex.len() >= 6 {
        let val = u32::from_str_radix(hex, 16).unwrap_or(0xCBA6F7);
        (((val >> 16) & 0xFF) as u8, ((val >> 8) & 0xFF) as u8, (val & 0xFF) as u8)
    } else {
        (203, 166, 247) // Default purple
    };
    
    // Draw 3 rounded squares representing dock icons
    let square_size = 7;
    let gap = 3;
    let total_width = square_size * 3 + gap * 2;
    let start_x = (SIZE - total_width) / 2;
    let start_y = (SIZE - square_size) / 2;
    
    for i in 0..3 {
        let sx = start_x + i * (square_size + gap);
        for dy in 0..square_size {
            for dx in 0..square_size {
                let x = sx + dx;
                let y = start_y + dy;
                if x < SIZE && y < SIZE {
                    let idx = (y * SIZE + x) * 4;
                    // Slight rounded corners effect
                    let is_corner = (dx == 0 || dx == square_size - 1) && (dy == 0 || dy == square_size - 1);
                    if !is_corner {
                        rgba[idx] = r;
                        rgba[idx + 1] = g;
                        rgba[idx + 2] = b;
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }
    
    tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)
}

struct DockApp {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    config: Config,
    renderer: Option<Renderer>,
    hovered_item: Option<usize>,
    running_states: Vec<bool>,
    last_process_check: Instant,
    cursor_in_window: bool,
    
    // Animation state
    dock_y_current: f32,
    dock_y_target: f32,
    dock_y_hidden: f32,
    dock_y_visible: f32,
    hide_timer: Option<Instant>,
    icon_scales: Vec<f32>,
    
    // Cursor position for smooth wave effect
    cursor_x: f32,
    cursor_y: f32,
    
    // Drag and drop state
    dragging: bool,
    drag_start_idx: Option<usize>,
    drag_start_x: f32,
    
    // Screen info
    screen_width: u32,
    screen_height: u32,
    
    // Tray
    _tray: Option<tray_icon::TrayIcon>,
    quit_id: Option<tray_icon::menu::MenuId>,
    
    // Hot reload
    config_path: PathBuf,
    config_rx: Option<mpsc::Receiver<Result<Event, notify::Error>>>,
    _watcher: Option<notify::RecommendedWatcher>,
    needs_reload: bool,
    last_config_modified: Option<SystemTime>,
    last_config_poll: Instant,
    
    // Tooltip
    tooltip: Option<Tooltip>,
    
    // Taskbar state
    taskbar_hidden: bool,
}

impl DockApp {
    fn new(config: Config, config_path: PathBuf) -> Self {
        let n = config.items.len();
        
        // Canonicalize path for reliable file watching
        let config_path = config_path.canonicalize().unwrap_or(config_path);
        
        // Set up file watcher for hot reload
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }).ok();
        
        Self {
            window: None,
            surface: None,
            config,
            renderer: None,
            hovered_item: None,
            running_states: Vec::new(),
            last_process_check: Instant::now() - PROCESS_CHECK_INTERVAL,
            cursor_in_window: false,
            dock_y_current: 0.0,
            dock_y_target: 0.0,
            dock_y_hidden: 0.0,
            dock_y_visible: 0.0,
            hide_timer: None,
            icon_scales: vec![1.0; n],
            cursor_x: -1000.0,
            cursor_y: -1000.0,
            dragging: false,
            drag_start_idx: None,
            drag_start_x: 0.0,
            screen_width: 1920,
            screen_height: 1080,
            _tray: None,
            quit_id: None,
            config_path,
            config_rx: Some(rx),
            _watcher: watcher,
            needs_reload: false,
            last_config_modified: None,
            last_config_poll: Instant::now(),
            tooltip: None,
            taskbar_hidden: false,
        }
    }
    
    fn start_watching(&mut self) {
        if let Some(watcher) = &mut self._watcher {
            if let Err(e) = watcher.watch(&self.config_path, RecursiveMode::NonRecursive) {
                eprintln!("Failed to watch config: {}", e);
            }
        }
    }
    
    fn check_config_reload(&mut self) {
        // Check notify watcher events
        if let Some(rx) = &self.config_rx {
            while let Ok(event) = rx.try_recv() {
                if let Ok(Event { kind: EventKind::Modify(_), .. }) = event {
                    self.needs_reload = true;
                }
            }
        }
        
        // Fallback: poll file modification time every 500ms
        if self.last_config_poll.elapsed() >= Duration::from_millis(500) {
            self.last_config_poll = Instant::now();
            if let Ok(meta) = std::fs::metadata(&self.config_path) {
                if let Ok(modified) = meta.modified() {
                    if let Some(last) = self.last_config_modified {
                        if modified > last {
                            self.needs_reload = true;
                        }
                    }
                    self.last_config_modified = Some(modified);
                }
            }
        }
    }
    
    fn reload_config(&mut self) {
        if !self.needs_reload {
            return;
        }
        self.needs_reload = false;
        
        // Small delay to let file finish writing
        std::thread::sleep(Duration::from_millis(50));
        
        if let Ok(new_config) = Config::load(&self.config_path) {
            let n = new_config.items.len();
            self.config = new_config;
            
            // Rebuild renderer with new config
            if let Ok(renderer) = Renderer::new(&self.config, &self.config.items) {
                // Resize window if needed
                if let Some(window) = &self.window {
                    let _ = window.request_inner_size(PhysicalSize::new(renderer.width, renderer.height));
                    
                    // Reposition with vertical offset
                    let x = (self.screen_width - renderer.width) / 2;
                    let offset = self.config.dock.negative_vertical_offset;
                    let y_vis = (self.screen_height as i32 - renderer.height as i32 + offset) as u32;
                    self.dock_y_visible = y_vis as f32;
                    self.dock_y_hidden = (self.screen_height + 20) as f32;
                    self.dock_y_target = y_vis as f32;
                    self.dock_y_current = y_vis as f32;
                    window.set_outer_position(PhysicalPosition::new(x as i32, y_vis as i32));
                    
                    // Request redraw to ensure window updates
                    window.request_redraw();
                }
                
                // Resize surface
                if let Some(surface) = &mut self.surface {
                    let _ = surface.resize(
                        NonZeroU32::new(renderer.width).unwrap(),
                        NonZeroU32::new(renderer.height).unwrap(),
                    );
                }
                
                self.renderer = Some(renderer);
            }
            
            self.running_states = vec![false; n];
            self.icon_scales = vec![1.0; n];
            self.last_process_check = Instant::now() - PROCESS_CHECK_INTERVAL;
            
            // Show dock after reload and prevent immediate hiding
            // Give user time to see the changes (2 seconds grace period)
            self.dock_y_target = self.dock_y_visible;
            self.hide_timer = None;
        }
    }

    fn redraw(&mut self) {
        // Prepare drag state for rendering (before borrowing surface)
        let drag_state = if self.dragging {
            self.drag_start_idx.map(|idx| (idx, self.get_drop_index(), self.cursor_x))
        } else {
            None
        };
        
        let Some(surface) = &mut self.surface else { return };
        let Some(renderer) = &self.renderer else { return };

        let mut buffer = surface.buffer_mut().unwrap();
        
        renderer.render(
            &mut buffer,
            &self.config.items,
            &self.running_states,
            self.hovered_item,
            &self.icon_scales,
            drag_state,
        );

        let _ = buffer.present();
    }

    fn update_running_states(&mut self) {
        if self.last_process_check.elapsed() < PROCESS_CHECK_INTERVAL {
            return;
        }
        self.last_process_check = Instant::now();

        let running = app_monitor::get_running_executables();
        self.running_states = self.config.items
            .iter()
            .map(|item| app_monitor::is_running(&item.path, &running))
            .collect();
    }

    fn launch_item(&self, index: usize) {
        if let Some(item) = self.config.items.get(index) {
            // Handle special system items
            if let Some(special) = &item.special {
                self.launch_special(special);
                return;
            }
            
            // Regular app launch
            if item.path.as_os_str().is_empty() {
                return;
            }
            
            // Try to focus existing window first
            if window_focus::focus_existing_window(&item.path) {
                return;
            }
            
            // No existing window found, launch new instance
            let mut cmd = Command::new(&item.path);
            if !item.args.is_empty() {
                cmd.args(&item.args);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            let _ = cmd.spawn();
        }
    }
    
    #[cfg(windows)]
    fn launch_special(&self, special: &str) {
        use std::os::windows::process::CommandExt;
        
        match special {
            "start_menu" => {
                // Ctrl+Esc opens Start Menu
                let script = r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^{ESC}')"#;
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "recycle_bin" => {
                let _ = Command::new("explorer")
                    .arg("shell:RecycleBinFolder")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "settings" => {
                let _ = Command::new("cmd")
                    .args(["/c", "start", "ms-settings:"])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "show_desktop" => {
                // Use Shell.Application COM object
                let _ = Command::new("powershell")
                    .args(["-Command", "(New-Object -ComObject Shell.Application).ToggleDesktop()"])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "task_view" => {
                // Use explorer shell command for task view
                let _ = Command::new("explorer")
                    .arg("shell:::{3080F90E-D7AD-11D9-BD98-0000947B0257}")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "action_center" | "notification_center" => {
                // Open notification center / action center
                let _ = Command::new("explorer")
                    .arg("ms-actioncenter:")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "system_tray" => {
                // Focus system tray with Win+B, then press Enter to open overflow
                let script = r#"$sig = '[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);'; $kb = Add-Type -MemberDefinition $sig -Name KB2 -PassThru; $kb::keybd_event(0x5B,0,0,0); $kb::keybd_event(0x42,0,0,0); $kb::keybd_event(0x42,0,2,0); $kb::keybd_event(0x5B,0,2,0); Start-Sleep -Milliseconds 100; $kb::keybd_event(0x0D,0,0,0); $kb::keybd_event(0x0D,0,2,0)"#;
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "quick_settings" => {
                // Open Windows 11 Quick Settings with Win+A
                let script = r#"$sig = '[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);'; $kb = Add-Type -MemberDefinition $sig -Name KB -PassThru; $kb::keybd_event(0x5B,0,0,0); $kb::keybd_event(0x41,0,0,0); $kb::keybd_event(0x41,0,2,0); $kb::keybd_event(0x5B,0,2,0)"#;
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "file_explorer" => {
                let _ = Command::new("explorer")
                    .arg(",")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "control_panel" => {
                let _ = Command::new("control")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "run_dialog" => {
                // Open Run dialog
                let _ = Command::new("rundll32")
                    .args(["shell32.dll,#61"])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "this_pc" | "my_computer" => {
                let _ = Command::new("explorer")
                    .arg("shell:MyComputerFolder")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "documents" => {
                let _ = Command::new("explorer")
                    .arg("shell:Personal")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "downloads" => {
                let _ = Command::new("explorer")
                    .arg("shell:Downloads")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "network" => {
                let _ = Command::new("explorer")
                    .arg("shell:NetworkPlacesFolder")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            "user_folder" | "home" => {
                let _ = Command::new("explorer")
                    .arg("shell:UsersFilesFolder")
                    .creation_flags(0x08000000)
                    .spawn();
            }
            _ => {
                eprintln!("Unknown special item: {}", special);
            }
        }
    }
    
    #[cfg(not(windows))]
    fn launch_special(&self, special: &str) {
        eprintln!("Special items not supported on this platform: {}", special);
    }
    
    /// Empty the Windows recycle bin
    #[cfg(windows)]
    fn empty_recycle_bin(&self) {
        use std::os::windows::process::CommandExt;
        
        // Use PowerShell to empty the recycle bin with confirmation
        let script = r#"Clear-RecycleBin -Force -Confirm:$false"#;
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
            .creation_flags(0x08000000)
            .spawn();
    }
    
    #[cfg(not(windows))]
    fn empty_recycle_bin(&self) {
        eprintln!("Empty recycle bin not supported on this platform");
    }

    fn update_animations(&mut self) -> bool {
        let mut animating = false;
        
        // Smooth dock Y position
        let dy = self.dock_y_target - self.dock_y_current;
        if dy.abs() > 0.5 {
            self.dock_y_current += dy * 0.15;
            if let Some(window) = &self.window {
                let x = ((self.screen_width as f32 - self.renderer.as_ref().unwrap().width as f32) / 2.0) as i32;
                window.set_outer_position(PhysicalPosition::new(x, self.dock_y_current as i32));
            }
            animating = true;
        }

        // Smooth wave magnification based on cursor distance (like macOS Dock)
        if let Some(renderer) = &self.renderer {
            let icon_size = renderer.icon_size as f32;
            let spacing_x = renderer.spacing.x as f32;
            let padding_left = renderer.padding.left as f32;
            
            // Wider range for wave effect - affects more neighbors
            let mag_range = icon_size * 3.5; 
            let max_scale = self.config.dock.magnification;
            
            for i in 0..self.icon_scales.len() {
                // Calculate icon center X position
                let icon_center_x = padding_left + (i as f32 * (icon_size + spacing_x)) + icon_size / 2.0;
                
                let target = if self.cursor_in_window && self.cursor_x >= 0.0 && !self.dragging {
                    // Distance from cursor to icon center
                    let dist = (self.cursor_x - icon_center_x).abs();
                    
                    if dist < mag_range {
                        // Smoother wave using cosine function for natural falloff
                        let t = dist / mag_range;
                        // Cosine curve gives a nice smooth wave effect
                        let falloff = (1.0 + (t * std::f32::consts::PI).cos()) / 2.0;
                        1.0 + (max_scale - 1.0) * falloff
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
                
                let d = target - self.icon_scales[i];
                if d.abs() > 0.001 {
                    // Slightly faster interpolation for more responsive feel
                    self.icon_scales[i] += d * 0.3;
                    animating = true;
                } else {
                    self.icon_scales[i] = target;
                }
            }
        }
        
        animating
    }

    fn check_hide(&mut self) {
        if !self.config.dock.auto_hide {
            return;
        }
        if let Some(t) = self.hide_timer {
            if t.elapsed() >= HIDE_DELAY {
                self.dock_y_target = self.dock_y_hidden;
                self.hide_timer = None;
            }
        }
    }

    fn show_dock(&mut self) {
        self.dock_y_target = self.dock_y_visible;
        self.hide_timer = None;
    }

    fn start_hide(&mut self) {
        if self.config.dock.auto_hide && self.hide_timer.is_none() {
            self.hide_timer = Some(Instant::now());
        }
    }

    fn setup_tray(&mut self) {
        let menu = Menu::new();
        let quit = MenuItem::new("Quit rDock", true, None);
        let qid = quit.id().clone();
        let _ = menu.append(&quit);
        
        // Create a dock-like tray icon (3 dots/squares)
        let icon = create_tray_icon(&self.config.dock.indicator_color);
        if let Ok(icon) = icon {
            if let Ok(tray) = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("rDock")
                .with_icon(icon)
                .build()
            {
                self._tray = Some(tray);
                self.quit_id = Some(qid);
            }
        }
    }
    
    fn handle_right_click(&mut self, _position: PhysicalPosition<f64>, event_loop: &ActiveEventLoop) {
        use context_menu::{show_context_menu, ContextMenuAction};
        use item_editor::{show_item_editor, DialogResult};
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        
        let Some(window) = &self.window else { return };
        
        // Get HWND
        let hwnd = match window.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as isize,
            _ => return,
        };
        
        // Get screen coordinates and convert to window-local for hit test
        let (screen_x, screen_y, local_x, local_y) = unsafe {
            let mut point = std::mem::zeroed::<windows::Win32::Foundation::POINT>();
            windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).ok();
            let screen_x = point.x;
            let screen_y = point.y;
            
            // Convert to window-local coordinates
            let hwnd_handle = windows::Win32::Foundation::HWND(hwnd as *mut _);
            windows::Win32::Graphics::Gdi::ScreenToClient(hwnd_handle, &mut point);
            (screen_x, screen_y, point.x, point.y)
        };
        
        // Perform hit test at click time using window-local cursor position
        let clicked_item = if let Some(renderer) = &self.renderer {
            renderer.hit_test(local_x, local_y, &self.config.items)
        } else {
            None
        };
        
        // Check if clicked item is a separator
        let is_separator = clicked_item
            .and_then(|i| self.config.items.get(i))
            .map(|item| item.is_separator())
            .unwrap_or(false);
        
        // Check if clicked item is a recycle bin
        let is_recycle_bin = clicked_item
            .and_then(|i| self.config.items.get(i))
            .and_then(|item| item.special.as_ref())
            .map(|special| special == "recycle_bin")
            .unwrap_or(false);
        
        // Show unified context menu
        let action = show_context_menu(hwnd, screen_x, screen_y, clicked_item, self.config.dock.locked, is_separator, is_recycle_bin);
        
        match action {
            ContextMenuAction::AddItem => {
                // Open item editor for new item
                match show_item_editor(None, true) {
                    DialogResult::Ok(item) => {
                        self.config.items.push(item);
                        self.save_config();
                        self.needs_reload = true;
                    }
                    _ => {}
                }
            }
            ContextMenuAction::AddSeparator => {
                self.config.items.push(DockItem::new_separator());
                self.save_config();
                self.needs_reload = true;
            }
            ContextMenuAction::AddSpecial(special_type) => {
                // Open item editor pre-filled with special type
                let name = context_menu::SPECIAL_ITEMS.iter()
                    .find(|(id, _)| *id == special_type)
                    .map(|(_, name)| name.to_string())
                    .unwrap_or_else(|| special_type.clone());
                
                let prefilled = DockItem {
                    name,
                    path: PathBuf::new(),
                    icon: None,
                    args: Vec::new(),
                    separator: false,
                    special: Some(special_type),
                };
                
                match show_item_editor(Some(&prefilled), true) {
                    DialogResult::Ok(item) => {
                        self.config.items.push(item);
                        self.save_config();
                        self.needs_reload = true;
                    }
                    _ => {}
                }
            }
            ContextMenuAction::RemoveItem(idx) => {
                if idx < self.config.items.len() {
                    self.config.items.remove(idx);
                    self.save_config();
                    self.needs_reload = true;
                }
            }
            ContextMenuAction::EditItem(idx) => {
                // Open item editor for existing item
                if idx < self.config.items.len() {
                    let existing = self.config.items[idx].clone();
                    match show_item_editor(Some(&existing), false) {
                        DialogResult::Ok(item) => {
                            self.config.items[idx] = item;
                            self.save_config();
                            self.needs_reload = true;
                        }
                        DialogResult::Remove => {
                            self.config.items.remove(idx);
                            self.save_config();
                            self.needs_reload = true;
                        }
                        DialogResult::Cancel => {}
                    }
                }
            }
            ContextMenuAction::ToggleLock => {
                self.config.dock.locked = !self.config.dock.locked;
                self.save_config();
            }
            ContextMenuAction::OpenConfig => {
                // Open config in default editor
                let _ = Command::new("cmd")
                    .args(["/c", "start", "", self.config_path.to_str().unwrap_or("")])
                    .spawn();
            }
            ContextMenuAction::EmptyRecycleBin => {
                self.empty_recycle_bin();
            }
            ContextMenuAction::Quit => {
                event_loop.exit();
            }
            ContextMenuAction::None => {}
        }
    }
    
    fn save_config(&self) {
        if let Err(e) = self.config.save(&self.config_path) {
            eprintln!("Failed to save config: {}", e);
        }
    }
    
    fn is_animating(&self) -> bool {
        // Check if dock position is animating
        let dock_animating = (self.dock_y_target - self.dock_y_current).abs() > 0.5;
        
        // Check if any icon scale is animating
        let icons_animating = self.icon_scales.iter().any(|&scale| (scale - 1.0).abs() > 0.01);
        
        // Check if hide timer is active
        let hide_pending = self.hide_timer.is_some();
        
        dock_animating || icons_animating || hide_pending || self.cursor_in_window
    }
    
    fn get_drop_index(&self) -> usize {
        // Calculate which position the cursor is over for dropping
        let Some(renderer) = &self.renderer else { return 0 };
        
        let icon_size = renderer.icon_size as f32;
        let spacing = renderer.spacing.x as f32;
        let padding_left = renderer.padding.left as f32;
        let num_items = self.config.items.len();
        
        if num_items == 0 {
            return 0;
        }
        
        // Calculate relative position within the icons area
        let rel_x = self.cursor_x - padding_left;
        
        // Each item slot is (icon_size + spacing) wide, last one just icon_size
        let slot_width = icon_size + spacing;
        
        if rel_x <= 0.0 {
            return 0;
        }
        
        // Find which slot we're in
        let slot = (rel_x / slot_width) as usize;
        let within_slot = rel_x - (slot as f32 * slot_width);
        
        // If we're in the right half of the slot, drop after this item
        if within_slot > icon_size / 2.0 {
            (slot + 1).min(num_items)
        } else {
            slot.min(num_items)
        }
    }
}

impl ApplicationHandler for DockApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .expect("No monitor found");
        
        let screen = monitor.size();
        self.screen_width = screen.width;
        self.screen_height = screen.height;

        let renderer = Renderer::new(&self.config, &self.config.items)
            .expect("Failed to create renderer");
        
        let dock_w = renderer.width;
        let dock_h = renderer.height;

        let x = (screen.width - dock_w) / 2;
        let offset = self.config.dock.negative_vertical_offset;
        // Positive offset = move down (bury into edge)
        let y_vis = (screen.height as i32 - dock_h as i32 + offset) as u32;
        // When hidden, keep 2 pixels visible at bottom edge so we can detect cursor
        let y_hid = screen.height - 2;
        
        self.dock_y_visible = y_vis as f32;
        self.dock_y_hidden = y_hid as f32;
        self.dock_y_current = y_vis as f32;
        self.dock_y_target = y_vis as f32;

        let attrs = Window::default_attributes()
            .with_title("rDock")
            .with_inner_size(PhysicalSize::new(dock_w, dock_h))
            .with_position(PhysicalPosition::new(x as i32, y_vis as i32))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_skip_taskbar(true);

        let window = Rc::new(event_loop.create_window(attrs).unwrap());
        
        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&ctx, window.clone()).unwrap();
        surface.resize(NonZeroU32::new(dock_w).unwrap(), NonZeroU32::new(dock_h).unwrap()).unwrap();

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.running_states = vec![false; self.config.items.len()];
        self.icon_scales = vec![1.0; self.config.items.len()];
        
        self.setup_tray();
        self.start_watching();
        
        // Initialize tooltip with config background color
        if let Some(window) = &self.window {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(RawWindowHandle::Win32(h)) = window.window_handle().map(|h| h.as_raw()) {
                let hwnd = windows::Win32::Foundation::HWND(h.hwnd.get() as *mut _);
                self.tooltip = Tooltip::new_with_color(hwnd, &self.config.dock.background_color);
            }
        }
        
        // Hide Windows taskbar if configured
        if self.config.dock.hide_windows_taskbar && !self.taskbar_hidden {
            set_taskbar_visibility(false);
            self.taskbar_hidden = true;
        }
    }
    
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Restore taskbar when exiting
        if self.taskbar_hidden {
            set_taskbar_visibility(true);
            self.taskbar_hidden = false;
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                self.check_config_reload();
                self.reload_config();
                self.update_running_states();
                self.check_hide();
                let _ = self.update_animations();
                self.redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_in_window = true;
                self.cursor_x = position.x as f32;
                self.cursor_y = position.y as f32;
                self.show_dock();
                
                // Check if we should start dragging (mouse moved enough while button held)
                if !self.dragging && self.drag_start_idx.is_some() && !self.config.dock.locked {
                    let dx = (self.cursor_x - self.drag_start_x).abs();
                    if dx > 5.0 {
                        // Start actual drag
                        self.dragging = true;
                    }
                }
                
                if !self.dragging {
                    if let Some(renderer) = &self.renderer {
                        let new_hovered = renderer.hit_test(
                            position.x as i32,
                            position.y as i32,
                            &self.config.items,
                        );
                        self.hovered_item = new_hovered;
                        
                        // Update tooltip
                        if let Some(tooltip) = &mut self.tooltip {
                            if let Some(idx) = new_hovered {
                                if let Some(item) = self.config.items.get(idx) {
                                    if !item.is_separator() && !item.name.is_empty() {
                                        // Get screen position for tooltip
                                        if let Some(window) = &self.window {
                                            let win_pos = window.outer_position().unwrap_or_default();
                                            let screen_x = win_pos.x + position.x as i32;
                                            let screen_y = win_pos.y;
                                            tooltip.show(&item.name, screen_x, screen_y);
                                        }
                                    } else {
                                        tooltip.hide();
                                    }
                                }
                            } else {
                                tooltip.hide();
                            }
                        }
                    }
                }
            }

            WindowEvent::CursorLeft { .. } => {
                self.cursor_in_window = false;
                self.cursor_x = -1000.0;
                self.cursor_y = -1000.0;
                self.hovered_item = None;
                // Cancel any drag in progress
                self.dragging = false;
                self.drag_start_idx = None;
                self.start_hide();
                // Hide tooltip
                if let Some(tooltip) = &mut self.tooltip {
                    tooltip.hide();
                }
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                // Start potential drag if unlocked and over an item
                if !self.config.dock.locked {
                    if let Some(idx) = self.hovered_item {
                        // Don't allow dragging separators by themselves in a special way
                        self.drag_start_idx = Some(idx);
                        self.drag_start_x = self.cursor_x;
                    }
                }
            }
            
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                if self.dragging {
                    // Complete the drag - reorder items
                    if let Some(from_idx) = self.drag_start_idx {
                        let to_idx = self.get_drop_index();
                        if to_idx != from_idx && to_idx != from_idx + 1 {
                            // Remove from old position and insert at new position
                            let item = self.config.items.remove(from_idx);
                            let insert_idx = if to_idx > from_idx { to_idx - 1 } else { to_idx };
                            self.config.items.insert(insert_idx, item);
                            self.save_config();
                            self.needs_reload = true;
                        }
                    }
                    self.dragging = false;
                    self.drag_start_idx = None;
                } else if self.drag_start_idx.is_some() {
                    // Was a click, not a drag - launch the item
                    if let Some(index) = self.hovered_item {
                        // Don't launch separators
                        if !self.config.items.get(index).map(|i| i.is_separator()).unwrap_or(false) {
                            self.launch_item(index);
                        }
                    }
                    self.drag_start_idx = None;
                }
            }
            
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                // Cancel any drag
                self.dragging = false;
                self.drag_start_idx = None;
                // Get cursor position for context menu
                let pos = PhysicalPosition::new(self.cursor_x as f64, self.cursor_y as f64);
                self.handle_right_click(pos, event_loop);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Handle tray menu
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.quit_id.as_ref() {
                event_loop.exit();
                return;
            }
        }

        // Check if we need to animate
        let needs_animation = self.is_animating();
        let needs_process_check = self.last_process_check.elapsed() >= PROCESS_CHECK_INTERVAL;
        let needs_config_check = self.last_config_poll.elapsed() >= Duration::from_millis(500);
        
        if needs_animation {
            // Animating - run at 60fps
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + ANIMATION_FRAME_TIME
            ));
        } else if needs_process_check || needs_config_check || self.needs_reload {
            // Need to check something - do it now then wait
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(100)
            ));
        } else {
            // Idle - wait for events, but wake up periodically to check processes
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + PROCESS_CHECK_INTERVAL
            ));
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    // Load config - check next to exe first, then current dir
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    
    let (config, config_path) = if let Some(dir) = &exe_dir {
        let exe_config = dir.join("config.toml");
        if exe_config.exists() {
            (Config::load(&exe_config)?, exe_config)
        } else {
            let local = std::path::PathBuf::from("config.toml");
            if local.exists() {
                (Config::load(&local)?, local)
            } else {
                eprintln!("No config.toml found at {} or current directory", exe_config.display());
                std::process::exit(1);
            }
        }
    } else {
        let local = std::path::PathBuf::from("config.toml");
        if local.exists() {
            (Config::load(&local)?, local)
        } else {
            eprintln!("No config.toml found. Please create one.");
            std::process::exit(1);
        }
    };

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = DockApp::new(config, config_path);
    event_loop.run_app(&mut app)?;

    Ok(())
}
