use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, BOOL, LPARAM, MAX_PATH};
use windows::Win32::System::ProcessStatus::{EnumProcesses, GetModuleFileNameExW};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_TERMINATE, TerminateProcess};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE};

/// Get list of currently running executable paths
pub fn get_running_executables() -> HashSet<PathBuf> {
    let mut running = HashSet::new();
    let mut pids: [u32; 2048] = [0; 2048];
    let mut bytes_returned: u32 = 0;

    unsafe {
        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_returned,
        ).is_ok() {
            let num_pids = bytes_returned as usize / std::mem::size_of::<u32>();
            
            for &pid in &pids[..num_pids] {
                if pid == 0 {
                    continue;
                }
                
                if let Some(path) = get_process_path(pid) {
                    running.insert(path);
                }
            }
        }
    }

    running
}

fn get_process_path(pid: u32) -> Option<PathBuf> {
    unsafe {
        let handle: HANDLE = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        ).ok()?;

        let mut buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let len = GetModuleFileNameExW(handle, None, &mut buffer);
        
        let _ = CloseHandle(handle);

        if len == 0 {
            return None;
        }

        let path = OsString::from_wide(&buffer[..len as usize]);
        Some(PathBuf::from(path))
    }
}

/// Check if a specific executable is running
pub fn is_running(exe_path: &Path, running: &HashSet<PathBuf>) -> bool {
    // Normalize path for comparison
    let normalized = exe_path.to_string_lossy().to_lowercase();

    running.iter().any(|p| p.to_string_lossy().to_lowercase() == normalized)
}

/// Gracefully quit all instances of an application by sending WM_CLOSE to its windows.
/// Falls back to TerminateProcess if no windows are found.
pub fn quit_application(exe_path: &Path) {
    let normalized = exe_path.to_string_lossy().to_lowercase();
    
    // Find all PIDs matching this executable
    let mut target_pids: HashSet<u32> = HashSet::new();
    let mut pids: [u32; 2048] = [0; 2048];
    let mut bytes_returned: u32 = 0;
    
    unsafe {
        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_returned,
        ).is_ok() {
            let num_pids = bytes_returned as usize / std::mem::size_of::<u32>();
            for &pid in &pids[..num_pids] {
                if pid == 0 { continue; }
                if let Some(path) = get_process_path(pid) {
                    if path.to_string_lossy().to_lowercase() == normalized {
                        target_pids.insert(pid);
                    }
                }
            }
        }
    }
    
    if target_pids.is_empty() {
        return;
    }
    
    // Send WM_CLOSE to all visible windows belonging to these processes
    let mut closed_any = false;
    let mut windows: Vec<(HWND, u32)> = Vec::new();
    
    unsafe {
        let _ = EnumWindows(
            Some(enum_close_callback),
            LPARAM(&mut windows as *mut Vec<(HWND, u32)> as isize),
        );
    }
    
    for (hwnd, pid) in &windows {
        if target_pids.contains(pid) {
            unsafe {
                let _ = PostMessageW(*hwnd, WM_CLOSE, None, None);
            }
            closed_any = true;
        }
    }
    
    // If no windows were found, force-terminate the processes
    if !closed_any {
        for pid in &target_pids {
            unsafe {
                if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, *pid) {
                    let _ = TerminateProcess(handle, 1);
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
}

unsafe extern "system" fn enum_close_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd).as_bool() {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            let windows = &mut *(lparam.0 as *mut Vec<(HWND, u32)>);
            windows.push((hwnd, pid));
        }
    }
    BOOL(1)
}
