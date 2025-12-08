//! Window focus utilities - find and activate existing app windows

use std::path::Path;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

/// Try to find and focus an existing window for the given executable path.
/// Returns true if a window was found and focused, false otherwise.
pub fn focus_existing_window(exe_path: &Path) -> bool {
    let exe_name = match exe_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.to_lowercase(),
        None => return false,
    };
    
    // Collect all visible top-level windows
    let mut windows: Vec<HWND> = Vec::new();
    
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut Vec<HWND> as isize),
        );
    }
    
    // Find a window belonging to our target process
    for hwnd in windows {
        if let Some(window_exe) = get_window_exe_name(hwnd) {
            if window_exe.to_lowercase() == exe_name {
                // Found a matching window - focus it
                focus_window(hwnd);
                return true;
            }
        }
    }
    
    false
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Only consider visible windows
    if IsWindowVisible(hwnd).as_bool() {
        // Skip windows with no title (usually background windows)
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len > 0 {
            let windows = &mut *(lparam.0 as *mut Vec<HWND>);
            windows.push(hwnd);
        }
    }
    BOOL(1) // Continue enumeration
}

fn get_window_exe_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        
        if pid == 0 {
            return None;
        }
        
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        
        let mut buf = [0u16; 260];
        let len = GetModuleFileNameExW(process, None, &mut buf);
        
        let _ = windows::Win32::Foundation::CloseHandle(process);
        
        if len == 0 {
            return None;
        }
        
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }
}

fn focus_window(hwnd: HWND) {
    unsafe {
        // If window is minimized, restore it
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        
        // Bring to foreground
        let _ = SetForegroundWindow(hwnd);
        
        // Also try BringWindowToTop for good measure
        let _ = BringWindowToTop(hwnd);
    }
}
