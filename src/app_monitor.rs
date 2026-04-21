use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, BOOL, LPARAM};
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
};

/// Snapshot of running executables. Paths are stored pre-lowercased so lookups
/// are O(1) without per-call allocation.
pub type RunningSet = HashSet<String>;

/// Enumerate running processes and return their executable paths (lowercased).
///
/// Uses `PROCESS_QUERY_LIMITED_INFORMATION` + `QueryFullProcessImageNameW`,
/// which is significantly cheaper than `PROCESS_VM_READ` + `GetModuleFileNameExW`
/// and also succeeds for elevated processes.
pub fn get_running_executables() -> RunningSet {
    let mut running = RunningSet::with_capacity(512);
    let mut pids: [u32; 2048] = [0; 2048];
    let mut bytes_returned: u32 = 0;

    unsafe {
        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_returned,
        )
        .is_err()
        {
            return running;
        }

        let num_pids = bytes_returned as usize / std::mem::size_of::<u32>();
        for &pid in &pids[..num_pids] {
            if pid == 0 {
                continue;
            }
            if let Some(path) = get_process_path_lower(pid) {
                running.insert(path);
            }
        }
    }

    running
}

/// Query a process's image path as a lowercased `String`. Returns `None` on
/// failure (access denied, protected process, exited, etc).
fn get_process_path_lower(pid: u32) -> Option<String> {
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;

    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buffer.as_mut_ptr()), &mut size).is_ok();
        let _ = CloseHandle(handle);

        if !ok || size == 0 {
            return None;
        }

        let os = OsString::from_wide(&buffer[..size as usize]);
        Some(os.to_string_lossy().to_lowercase())
    }
}

/// Check if a specific executable is in the running snapshot. O(1).
pub fn is_running(exe_path: &Path, running: &RunningSet) -> bool {
    let normalized = exe_path.to_string_lossy().to_lowercase();
    running.contains(&normalized)
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
                if let Some(path) = get_process_path_lower(pid) {
                    if path == normalized {
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
