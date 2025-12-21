//! System tray overflow popup
//! Opens the Windows 11 tray overflow using keyboard shortcut

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::core::PCWSTR;

pub fn show_tray_popup_at_cursor() {
    // Get cursor position before doing anything
    let mut cursor_pos = POINT { x: 0, y: 0 };
    unsafe { let _ = GetCursorPos(&mut cursor_pos); }
    
    std::thread::spawn(move || {
        unsafe {
            open_tray_overflow(cursor_pos);
        }
    });
}

unsafe fn open_tray_overflow(target_pos: POINT) {
    // Windows 11 uses XAML for the taskbar - no Win32 chevron button exists
    // Use Win+B to focus system tray, then Enter to open overflow
    
    let mut inputs = [
        // Win key down
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_LWIN,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // B key down
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0x42), // B
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // B key up
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0x42),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Win key up
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_LWIN,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    
    let _ = SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Press Enter to open overflow
    let mut enter_inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    
    let _ = SendInput(&mut enter_inputs, std::mem::size_of::<INPUT>() as i32);
    
    // Wait for overflow window to appear
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Find and move the overflow window near the cursor
    let overflow_class: Vec<u16> = "TopLevelWindowForOverflowXamlIsland\0".encode_utf16().collect();
    if let Ok(overflow) = FindWindowW(PCWSTR(overflow_class.as_ptr()), PCWSTR::null()) {
        if !overflow.0.is_null() && IsWindowVisible(overflow).as_bool() {
            let mut overflow_rect = RECT::default();
            if GetWindowRect(overflow, &mut overflow_rect).is_ok() {
                let width = overflow_rect.right - overflow_rect.left;
                let height = overflow_rect.bottom - overflow_rect.top;
                
                // Position above the cursor, centered horizontally
                let new_x = target_pos.x - width / 2;
                let new_y = target_pos.y - height - 20;
                
                let _ = SetWindowPos(
                    overflow,
                    HWND_TOPMOST,
                    new_x,
                    new_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
                
                // Move cursor into the window to keep it open
                let _ = SetCursorPos(target_pos.x, new_y + height / 2);
            }
        }
    }
}
