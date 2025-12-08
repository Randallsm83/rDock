//! Item editor dialog for dock items

use std::path::PathBuf;
use std::cell::RefCell;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, LRESULT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::config::DockItem;
use crate::context_menu::{pick_executable_with_path, pick_icon_with_path, SPECIAL_ITEMS};

// Control IDs
const ID_NAME_EDIT: i32 = 101;
const ID_PATH_EDIT: i32 = 102;
const ID_PATH_BROWSE: i32 = 103;
const ID_ICON_EDIT: i32 = 104;
const ID_ICON_BROWSE: i32 = 105;
const ID_ARGS_EDIT: i32 = 106;
const ID_SPECIAL_COMBO: i32 = 107;
const ID_OK: i32 = 1;
const ID_CANCEL: i32 = 2;
const ID_REMOVE: i32 = 108;

// Style constants
const SS_RIGHT: u32 = 0x0002;
const ES_AUTOHSCROLL: u32 = 0x0080;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CBS_HASSTRINGS: u32 = 0x0200;
const COLOR_BTNFACE: u32 = 15;

// Dialog result stored in thread-local for the dialog proc
thread_local! {
    static DIALOG_RESULT: RefCell<Option<DialogResult>> = const { RefCell::new(None) };
    static DIALOG_ITEM: RefCell<Option<DockItem>> = const { RefCell::new(None) };
    static DIALOG_IS_NEW: RefCell<bool> = const { RefCell::new(true) };
}

#[derive(Debug, Clone)]
pub enum DialogResult {
    Ok(DockItem),
    Remove,
    Cancel,
}

/// Show the item editor dialog
/// Returns DialogResult with the edited item, remove request, or cancel
pub fn show_item_editor(item: Option<&DockItem>, is_new: bool) -> DialogResult {
    // Initialize dialog item
    let initial_item = item.cloned().unwrap_or_else(|| DockItem {
        name: String::new(),
        path: PathBuf::new(),
        icon: None,
        args: Vec::new(),
        separator: false,
        special: None,
    });
    
    DIALOG_ITEM.with(|cell| {
        *cell.borrow_mut() = Some(initial_item);
    });
    DIALOG_RESULT.with(|cell| {
        *cell.borrow_mut() = None;
    });
    DIALOG_IS_NEW.with(|cell| {
        *cell.borrow_mut() = is_new;
    });
    
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0 as *mut _;
        let hinstance = windows::Win32::Foundation::HINSTANCE(hinstance);
        
        // Register window class
        let class_name: Vec<u16> = "RDockItemEditor\0".encode_utf16().collect();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(dialog_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: HICON::default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH((COLOR_BTNFACE + 1) as *mut _),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: HICON::default(),
        };
        
        RegisterClassExW(&wc);
        
        // Calculate window size and position
        let width = 580;
        let height = if is_new { 330 } else { 380 };
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - width) / 2;
        let y = (screen_h - height) / 2;
        
        let title: Vec<u16> = if is_new {
            "Add Item\0".encode_utf16().collect()
        } else {
            "Edit Item\0".encode_utf16().collect()
        };
        
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            x, y, width, height,
            HWND::default(),
            HMENU::default(),
            hinstance,
            None,
        ).unwrap_or_default();
        
        if hwnd.is_invalid() {
            return DialogResult::Cancel;
        }
        
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = unsafe { windows::Win32::Graphics::Gdi::UpdateWindow(hwnd) };
        
        // Modal message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            
            // Check if dialog was closed
            if !IsWindow(hwnd).as_bool() {
                break;
            }
        }
        
        // Get result
        DIALOG_RESULT.with(|cell| {
            cell.borrow_mut().take().unwrap_or(DialogResult::Cancel)
        })
    }
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let is_new = DIALOG_IS_NEW.with(|cell| *cell.borrow());
            create_controls(hwnd, is_new);
            populate_controls(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            handle_command(hwnd, id);
            LRESULT(0)
        }
        WM_CLOSE => {
            DIALOG_RESULT.with(|cell| {
                *cell.borrow_mut() = Some(DialogResult::Cancel);
            });
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn create_controls(hwnd: HWND, is_new: bool) {
    let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0 as *mut _;
    let hinstance = windows::Win32::Foundation::HINSTANCE(hinstance);
    
    let mut y = 20;
    let label_w = 90;
    let edit_x = 110;
    let edit_w = 340;
    let btn_w = 90;
    let btn_x = 460;
    let row_h = 35;
    
    let static_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let edit_class: Vec<u16> = "EDIT\0".encode_utf16().collect();
    let button_class: Vec<u16> = "BUTTON\0".encode_utf16().collect();
    let combo_class: Vec<u16> = "COMBOBOX\0".encode_utf16().collect();
    
    // Name
    let name_label: Vec<u16> = "Name:\0".encode_utf16().collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(static_class.as_ptr()), PCWSTR(name_label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_RIGHT),
        10, y + 3, label_w, 20, hwnd, HMENU::default(), hinstance, None
    );
    let _ = CreateWindowExW(
        WS_EX_CLIENTEDGE, PCWSTR(edit_class.as_ptr()), PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL),
        edit_x, y, edit_w + btn_w + 10, 24, hwnd, HMENU(ID_NAME_EDIT as *mut _), hinstance, None
    );
    y += row_h + 5;
    
    // Path
    let path_label: Vec<u16> = "Path:\0".encode_utf16().collect();
    let browse_text: Vec<u16> = "Browse...\0".encode_utf16().collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(static_class.as_ptr()), PCWSTR(path_label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_RIGHT),
        10, y + 3, label_w, 20, hwnd, HMENU::default(), hinstance, None
    );
    let _ = CreateWindowExW(
        WS_EX_CLIENTEDGE, PCWSTR(edit_class.as_ptr()), PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL),
        edit_x, y, edit_w, 24, hwnd, HMENU(ID_PATH_EDIT as *mut _), hinstance, None
    );
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(button_class.as_ptr()), PCWSTR(browse_text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        btn_x, y, btn_w, 24, hwnd, HMENU(ID_PATH_BROWSE as *mut _), hinstance, None
    );
    y += row_h + 5;
    
    // Icon
    let icon_label: Vec<u16> = "Icon:\0".encode_utf16().collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(static_class.as_ptr()), PCWSTR(icon_label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_RIGHT),
        10, y + 3, label_w, 20, hwnd, HMENU::default(), hinstance, None
    );
    let _ = CreateWindowExW(
        WS_EX_CLIENTEDGE, PCWSTR(edit_class.as_ptr()), PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL),
        edit_x, y, edit_w, 24, hwnd, HMENU(ID_ICON_EDIT as *mut _), hinstance, None
    );
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(button_class.as_ptr()), PCWSTR(browse_text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        btn_x, y, btn_w, 24, hwnd, HMENU(ID_ICON_BROWSE as *mut _), hinstance, None
    );
    y += row_h + 5;
    
    // Args
    let args_label: Vec<u16> = "Arguments:\0".encode_utf16().collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(static_class.as_ptr()), PCWSTR(args_label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_RIGHT),
        10, y + 3, label_w, 20, hwnd, HMENU::default(), hinstance, None
    );
    let _ = CreateWindowExW(
        WS_EX_CLIENTEDGE, PCWSTR(edit_class.as_ptr()), PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL),
        edit_x, y, edit_w + btn_w + 10, 24, hwnd, HMENU(ID_ARGS_EDIT as *mut _), hinstance, None
    );
    y += row_h + 5;
    
    // Special type dropdown
    let special_label: Vec<u16> = "Special:\0".encode_utf16().collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(static_class.as_ptr()), PCWSTR(special_label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_RIGHT),
        10, y + 3, label_w, 20, hwnd, HMENU::default(), hinstance, None
    );
    let combo = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(combo_class.as_ptr()), PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST | CBS_HASSTRINGS),
        edit_x, y, edit_w + btn_w + 10, 200, hwnd, HMENU(ID_SPECIAL_COMBO as *mut _), hinstance, None
    ).unwrap_or_default();
    
    // Populate combo box
    let none_text: Vec<u16> = "(None - Regular Item)\0".encode_utf16().collect();
    SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(none_text.as_ptr() as isize));
    
    for (_, display_name) in SPECIAL_ITEMS {
        let text: Vec<u16> = format!("{}\0", display_name).encode_utf16().collect();
        SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as isize));
    }
    
    SendMessageW(combo, CB_SETCURSEL, WPARAM(0), LPARAM(0));
    y += row_h + 15;
    
    // Buttons
    let ok_text: Vec<u16> = "OK\0".encode_utf16().collect();
    let cancel_text: Vec<u16> = "Cancel\0".encode_utf16().collect();
    let remove_text: Vec<u16> = "Remove\0".encode_utf16().collect();
    
    let btn_y = y + 10;
    
    if !is_new {
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE(0), PCWSTR(button_class.as_ptr()), PCWSTR(remove_text.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            15, btn_y, 80, 28, hwnd, HMENU(ID_REMOVE as *mut _), hinstance, None
        );
    }
    
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(button_class.as_ptr()), PCWSTR(ok_text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0x0001), // BS_DEFPUSHBUTTON
        370, btn_y, 90, 30, hwnd, HMENU(ID_OK as *mut _), hinstance, None
    );
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0), PCWSTR(button_class.as_ptr()), PCWSTR(cancel_text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        470, btn_y, 90, 30, hwnd, HMENU(ID_CANCEL as *mut _), hinstance, None
    );
}

unsafe fn populate_controls(hwnd: HWND) {
    DIALOG_ITEM.with(|cell| {
        if let Some(item) = cell.borrow().as_ref() {
            set_edit_text(hwnd, ID_NAME_EDIT, &item.name);
            set_edit_text(hwnd, ID_PATH_EDIT, &item.path.to_string_lossy());
            set_edit_text(hwnd, ID_ICON_EDIT, &item.icon.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
            set_edit_text(hwnd, ID_ARGS_EDIT, &item.args.join(" "));
            
            // Set special combo
            if let Ok(combo) = GetDlgItem(hwnd, ID_SPECIAL_COMBO) {
                if let Some(special) = &item.special {
                    if let Some(idx) = SPECIAL_ITEMS.iter().position(|(id, _)| id == special) {
                        SendMessageW(combo, CB_SETCURSEL, WPARAM(idx + 1), LPARAM(0));
                    }
                } else {
                    SendMessageW(combo, CB_SETCURSEL, WPARAM(0), LPARAM(0));
                }
            }
        }
    });
}

unsafe fn set_edit_text(hwnd: HWND, id: i32, text: &str) {
    if let Ok(ctrl) = GetDlgItem(hwnd, id) {
        let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = SetWindowTextW(ctrl, PCWSTR(text_wide.as_ptr()));
    }
}

unsafe fn get_edit_text(hwnd: HWND, id: i32) -> String {
    let Ok(ctrl) = GetDlgItem(hwnd, id) else { return String::new() };
    let len = GetWindowTextLengthW(ctrl) as usize;
    if len == 0 {
        return String::new();
    }
    let mut buf: Vec<u16> = vec![0; len + 1];
    GetWindowTextW(ctrl, &mut buf);
    String::from_utf16_lossy(&buf[..len])
}

unsafe fn handle_command(hwnd: HWND, id: i32) {
    match id {
        ID_PATH_BROWSE => {
            let current = get_edit_text(hwnd, ID_PATH_EDIT);
            let current_path = if current.is_empty() { None } else { Some(PathBuf::from(&current)) };
            if let Some(path) = pick_executable_with_path(current_path.as_ref()) {
                set_edit_text(hwnd, ID_PATH_EDIT, &path.to_string_lossy());
                let name = get_edit_text(hwnd, ID_NAME_EDIT);
                if name.is_empty() {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        set_edit_text(hwnd, ID_NAME_EDIT, stem);
                    }
                }
            }
        }
        ID_ICON_BROWSE => {
            let current = get_edit_text(hwnd, ID_ICON_EDIT);
            let current_path = if current.is_empty() { None } else { Some(PathBuf::from(&current)) };
            if let Some(path) = pick_icon_with_path(current_path.as_ref()) {
                set_edit_text(hwnd, ID_ICON_EDIT, &path.to_string_lossy());
            }
        }
        ID_OK => {
            let name = get_edit_text(hwnd, ID_NAME_EDIT);
            let path_str = get_edit_text(hwnd, ID_PATH_EDIT);
            let icon_str = get_edit_text(hwnd, ID_ICON_EDIT);
            let args_str = get_edit_text(hwnd, ID_ARGS_EDIT);
            
            let sel = if let Ok(combo) = GetDlgItem(hwnd, ID_SPECIAL_COMBO) {
                SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32
            } else {
                0
            };
            
            let special = if sel > 0 && (sel - 1) < SPECIAL_ITEMS.len() as i32 {
                Some(SPECIAL_ITEMS[(sel - 1) as usize].0.to_string())
            } else {
                None
            };
            
            let item = DockItem {
                name: if name.is_empty() { "Unnamed".to_string() } else { name },
                path: PathBuf::from(path_str),
                icon: if icon_str.is_empty() { None } else { Some(PathBuf::from(icon_str)) },
                args: if args_str.is_empty() { Vec::new() } else { shell_words::split(&args_str).unwrap_or_else(|_| vec![args_str]) },
                separator: false,
                special,
            };
            
            DIALOG_RESULT.with(|cell| {
                *cell.borrow_mut() = Some(DialogResult::Ok(item));
            });
            let _ = DestroyWindow(hwnd);
        }
        ID_CANCEL => {
            DIALOG_RESULT.with(|cell| {
                *cell.borrow_mut() = Some(DialogResult::Cancel);
            });
            let _ = DestroyWindow(hwnd);
        }
        ID_REMOVE => {
            let msg: Vec<u16> = "Remove this item from the dock?\0".encode_utf16().collect();
            let title: Vec<u16> = "Confirm Remove\0".encode_utf16().collect();
            let result = MessageBoxW(
                hwnd,
                PCWSTR(msg.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_YESNO | MB_ICONQUESTION
            );
            if result == IDYES {
                DIALOG_RESULT.with(|cell| {
                    *cell.borrow_mut() = Some(DialogResult::Remove);
                });
                let _ = DestroyWindow(hwnd);
            }
        }
        _ => {}
    }
}
