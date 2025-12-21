//! Context menu and file dialog handling for dock item management

use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Shell::{
    IFileDialog, IShellItem, FileOpenDialog, FileSaveDialog, FOS_FILEMUSTEXIST, FOS_PATHMUSTEXIST,
    FOS_OVERWRITEPROMPT, SIGDN_FILESYSPATH,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuAction {
    None,
    // Item-specific actions
    EditItem(usize),
    RemoveItem(usize),
    EmptyRecycleBin,
    // General actions
    AddItem,
    AddSeparator,
    AddSpecial(String),  // special item type
    ToggleLock,
    OpenConfig,
    SaveConfigAs,
    LoadConfig,
    ResetSettings,
    ResetAll,
    Quit,
}

const ID_EDIT_ITEM: u32 = 1001;
const ID_REMOVE_ITEM: u32 = 1003;
const ID_ADD_ITEM: u32 = 1004;
const ID_ADD_SEPARATOR: u32 = 1005;
const ID_TOGGLE_LOCK: u32 = 1006;
const ID_OPEN_CONFIG: u32 = 1007;
const ID_QUIT: u32 = 1008;
const ID_EMPTY_RECYCLE_BIN: u32 = 1009;
const ID_SAVE_CONFIG_AS: u32 = 1010;
const ID_LOAD_CONFIG: u32 = 1011;
const ID_RESET_SETTINGS: u32 = 1012;
const ID_RESET_ALL: u32 = 1013;

// Special item IDs start at 2000
const ID_SPECIAL_BASE: u32 = 2000;

/// List of all special items with (id, display_name)
pub const SPECIAL_ITEMS: &[(&str, &str)] = &[
    ("start_menu", "Start Menu"),
    ("settings", "Settings"),
    ("recycle_bin", "Recycle Bin"),
    ("show_desktop", "Show Desktop"),
    ("system_tray", "System Tray (Hidden Icons)"),
    ("quick_settings", "Quick Settings"),
    ("file_explorer", "File Explorer"),
    ("this_pc", "This PC"),
    ("documents", "Documents"),
    ("downloads", "Downloads"),
    ("user_folder", "User Folder"),
    ("network", "Network"),
    ("control_panel", "Control Panel"),
    ("task_view", "Task View"),
    ("action_center", "Action Center"),
    ("run_dialog", "Run Dialog"),
];

/// Show unified context menu
pub fn show_context_menu(hwnd: isize, x: i32, y: i32, item_index: Option<usize>, is_locked: bool, is_separator: bool, is_recycle_bin: bool) -> ContextMenuAction {
    unsafe {
        let hmenu = CreatePopupMenu().unwrap_or_default();
        if hmenu.is_invalid() {
            return ContextMenuAction::None;
        }

        // Item-specific options (only if clicked on an item and not locked)
        if let Some(_idx) = item_index {
            // Show "Empty Recycle Bin" for recycle bin special item (even when locked)
            if is_recycle_bin {
                let empty_text: Vec<u16> = "Empty Recycle Bin\0".encode_utf16().collect();
                let _ = AppendMenuW(hmenu, MF_STRING, ID_EMPTY_RECYCLE_BIN as usize, PCWSTR(empty_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            }
            
            if !is_locked {
                if !is_separator {
                    let edit_text: Vec<u16> = "Edit Item...\0".encode_utf16().collect();
                    let _ = AppendMenuW(hmenu, MF_STRING, ID_EDIT_ITEM as usize, PCWSTR(edit_text.as_ptr()));
                }
                let remove_text: Vec<u16> = "Remove\0".encode_utf16().collect();
                let _ = AppendMenuW(hmenu, MF_STRING, ID_REMOVE_ITEM as usize, PCWSTR(remove_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            }
        }

        // General options (always shown)
        if !is_locked {
            let add_text: Vec<u16> = "Add Item...\0".encode_utf16().collect();
            let sep_text: Vec<u16> = "Add Separator\0".encode_utf16().collect();
            let _ = AppendMenuW(hmenu, MF_STRING, ID_ADD_ITEM as usize, PCWSTR(add_text.as_ptr()));
            let _ = AppendMenuW(hmenu, MF_STRING, ID_ADD_SEPARATOR as usize, PCWSTR(sep_text.as_ptr()));
            
            // Create submenu for special items
            let hsubmenu = CreatePopupMenu().unwrap_or_default();
            if !hsubmenu.is_invalid() {
                for (i, (_, display_name)) in SPECIAL_ITEMS.iter().enumerate() {
                    let text: Vec<u16> = format!("{}\0", display_name).encode_utf16().collect();
                    let _ = AppendMenuW(hsubmenu, MF_STRING, (ID_SPECIAL_BASE + i as u32) as usize, PCWSTR(text.as_ptr()));
                }
                let special_text: Vec<u16> = "Add Special Item\0".encode_utf16().collect();
                let _ = AppendMenuW(hmenu, MF_POPUP, hsubmenu.0 as usize, PCWSTR(special_text.as_ptr()));
            }
            
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        }
        
        // Lock toggle
        let lock_text: Vec<u16> = if is_locked {
            "Unlock Icons\0".encode_utf16().collect()
        } else {
            "Lock Icons\0".encode_utf16().collect()
        };
        let _ = AppendMenuW(hmenu, MF_STRING, ID_TOGGLE_LOCK as usize, PCWSTR(lock_text.as_ptr()));
        
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        
        let config_text: Vec<u16> = "Edit Config...\0".encode_utf16().collect();
        let save_text: Vec<u16> = "Save Config As...\0".encode_utf16().collect();
        let load_text: Vec<u16> = "Load Config...\0".encode_utf16().collect();
        let reset_settings_text: Vec<u16> = "Reset Settings\0".encode_utf16().collect();
        let reset_all_text: Vec<u16> = "Reset All\0".encode_utf16().collect();
        let quit_text: Vec<u16> = "Quit\0".encode_utf16().collect();
        let _ = AppendMenuW(hmenu, MF_STRING, ID_OPEN_CONFIG as usize, PCWSTR(config_text.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_SAVE_CONFIG_AS as usize, PCWSTR(save_text.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_LOAD_CONFIG as usize, PCWSTR(load_text.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_RESET_SETTINGS as usize, PCWSTR(reset_settings_text.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_RESET_ALL as usize, PCWSTR(reset_all_text.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(hmenu, MF_STRING, ID_QUIT as usize, PCWSTR(quit_text.as_ptr()));

        // Required: set foreground window so menu dismisses properly on click outside
        let hwnd_handle = HWND(hwnd as *mut _);
        let _ = SetForegroundWindow(hwnd_handle);
        
        // Show menu - TPM_NONOTIFY prevents WM_COMMAND, we use TPM_RETURNCMD instead
        let cmd = TrackPopupMenu(
            hmenu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            x,
            y,
            0,
            hwnd_handle,
            None,
        );
        
        // Post a null message to ensure proper menu cleanup
        let _ = PostMessageW(hwnd_handle, WM_NULL, None, None);

        let _ = DestroyMenu(hmenu);

        let cmd_id = cmd.0 as u32;
        
        // Check if it's a special item
        if cmd_id >= ID_SPECIAL_BASE && cmd_id < ID_SPECIAL_BASE + SPECIAL_ITEMS.len() as u32 {
            let idx = (cmd_id - ID_SPECIAL_BASE) as usize;
            return ContextMenuAction::AddSpecial(SPECIAL_ITEMS[idx].0.to_string());
        }
        
        match cmd_id {
            ID_EDIT_ITEM => ContextMenuAction::EditItem(item_index.unwrap_or(0)),
            ID_REMOVE_ITEM => ContextMenuAction::RemoveItem(item_index.unwrap_or(0)),
            ID_EMPTY_RECYCLE_BIN => ContextMenuAction::EmptyRecycleBin,
            ID_ADD_ITEM => ContextMenuAction::AddItem,
            ID_ADD_SEPARATOR => ContextMenuAction::AddSeparator,
            ID_TOGGLE_LOCK => ContextMenuAction::ToggleLock,
            ID_OPEN_CONFIG => ContextMenuAction::OpenConfig,
            ID_SAVE_CONFIG_AS => ContextMenuAction::SaveConfigAs,
            ID_LOAD_CONFIG => ContextMenuAction::LoadConfig,
            ID_RESET_SETTINGS => ContextMenuAction::ResetSettings,
            ID_RESET_ALL => ContextMenuAction::ResetAll,
            ID_QUIT => ContextMenuAction::Quit,
            _ => ContextMenuAction::None,
        }
    }
}

/// Open file dialog to select an executable
pub fn pick_executable_with_path(initial: Option<&PathBuf>) -> Option<PathBuf> {
    pick_file(
        "Select Application",
        &[("Executables", "*.exe"), ("All Files", "*.*")],
        initial,
    )
}

/// Open file dialog to select an icon
pub fn pick_icon_with_path(initial: Option<&PathBuf>) -> Option<PathBuf> {
    pick_file(
        "Select Icon",
        &[("Icons", "*.ico;*.png"), ("ICO Files", "*.ico"), ("PNG Files", "*.png"), ("All Files", "*.*")],
        initial,
    )
}

fn pick_file(title: &str, filters: &[(&str, &str)], initial_path: Option<&PathBuf>) -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        
        let dialog: IFileDialog = match CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) {
            Ok(d) => d,
            Err(_) => {
                CoUninitialize();
                return None;
            }
        };

        // Set options
        if let Ok(opts) = dialog.GetOptions() {
            let _ = dialog.SetOptions(opts | FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST);
        }

        // Set title
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = dialog.SetTitle(PCWSTR(title_wide.as_ptr()));

        // Set initial folder if path provided
        if let Some(path) = initial_path {
            // Get the parent directory (works for both files and dirs)
            let folder = path.parent().map(|p| p.to_path_buf()).filter(|p| p.exists());
            
            if let Some(folder_path) = folder {
                // Strip \\?\ prefix if present (canonicalize adds it, but shell APIs don't like it)
                let folder_str = folder_path.to_string_lossy();
                let folder_str = folder_str.strip_prefix(r"\\?\")
                    .unwrap_or(&folder_str);
                let folder_wide: Vec<u16> = folder_str.encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(shell_item) = windows::Win32::UI::Shell::SHCreateItemFromParsingName::<_, _, IShellItem>(
                    PCWSTR(folder_wide.as_ptr()),
                    None,
                ) {
                    // Use SetFolder to force the folder (overrides remembered location)
                    let _ = dialog.SetFolder(&shell_item);
                }
            }
        }

        // Build filter spec
        let mut filter_specs = Vec::new();
        let mut filter_strings: Vec<(Vec<u16>, Vec<u16>)> = Vec::new();
        
        for (name, pattern) in filters {
            let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let pattern_wide: Vec<u16> = pattern.encode_utf16().chain(std::iter::once(0)).collect();
            filter_strings.push((name_wide, pattern_wide));
        }
        
        for (name, pattern) in &filter_strings {
            filter_specs.push(windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC {
                pszName: PCWSTR(name.as_ptr()),
                pszSpec: PCWSTR(pattern.as_ptr()),
            });
        }
        
        if !filter_specs.is_empty() {
            let _ = dialog.SetFileTypes(&filter_specs);
        }

        // Show dialog
        let result = if dialog.Show(HWND::default()).is_ok() {
            dialog.GetResult().ok().and_then(|item: IShellItem| {
                item.GetDisplayName(SIGDN_FILESYSPATH).ok().map(|path| {
                    let path_str = path.to_string().unwrap_or_default();
                    PathBuf::from(path_str)
                })
            })
        } else {
            None
        };

        CoUninitialize();
        result
    }
}

/// Open file dialog to select a config file to load
pub fn pick_config_file(initial_path: Option<&PathBuf>) -> Option<PathBuf> {
    pick_file(
        "Load Config",
        &[("TOML Config", "*.toml"), ("All Files", "*.*")],
        initial_path,
    )
}

/// Save file dialog to save config
pub fn save_config_dialog(initial_path: Option<&PathBuf>) -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        
        let dialog: IFileDialog = match CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER) {
            Ok(d) => d,
            Err(_) => {
                CoUninitialize();
                return None;
            }
        };

        // Set options
        if let Ok(opts) = dialog.GetOptions() {
            let _ = dialog.SetOptions(opts | FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST);
        }

        // Set title
        let title_wide: Vec<u16> = "Save Config As".encode_utf16().chain(std::iter::once(0)).collect();
        let _ = dialog.SetTitle(PCWSTR(title_wide.as_ptr()));
        
        // Set default extension
        let ext_wide: Vec<u16> = "toml".encode_utf16().chain(std::iter::once(0)).collect();
        let _ = dialog.SetDefaultExtension(PCWSTR(ext_wide.as_ptr()));
        
        // Set default filename
        let filename_wide: Vec<u16> = "config.toml".encode_utf16().chain(std::iter::once(0)).collect();
        let _ = dialog.SetFileName(PCWSTR(filename_wide.as_ptr()));

        // Set initial folder if path exists
        if let Some(path) = initial_path {
            let folder = path.parent().map(|p| p.to_path_buf());
            if let Some(folder_path) = folder {
                let folder_wide: Vec<u16> = folder_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(shell_item) = windows::Win32::UI::Shell::SHCreateItemFromParsingName::<_, _, IShellItem>(
                    PCWSTR(folder_wide.as_ptr()),
                    None,
                ) {
                    let _ = dialog.SetFolder(&shell_item);
                }
            }
        }

        // Build filter spec
        let filter_name: Vec<u16> = "TOML Config".encode_utf16().chain(std::iter::once(0)).collect();
        let filter_pattern: Vec<u16> = "*.toml".encode_utf16().chain(std::iter::once(0)).collect();
        let filter_specs = [windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC {
            pszName: PCWSTR(filter_name.as_ptr()),
            pszSpec: PCWSTR(filter_pattern.as_ptr()),
        }];
        let _ = dialog.SetFileTypes(&filter_specs);

        // Show dialog
        let result = if dialog.Show(HWND::default()).is_ok() {
            dialog.GetResult().ok().and_then(|item: IShellItem| {
                item.GetDisplayName(SIGDN_FILESYSPATH).ok().map(|path| {
                    let path_str = path.to_string().unwrap_or_default();
                    PathBuf::from(path_str)
                })
            })
        } else {
            None
        };

        CoUninitialize();
        result
    }
}

/// Simple input dialog for item name (uses a basic approach)
#[allow(dead_code)]
pub fn input_dialog(title: &str, _prompt: &str, default: &str) -> Option<String> {
    // For simplicity, we'll use a workaround - create a temp file approach
    // A proper implementation would use a custom dialog window
    // For now, return the default or a generated name
    if default.is_empty() {
        Some(format!("{} Item", title))
    } else {
        Some(default.to_string())
    }
}
