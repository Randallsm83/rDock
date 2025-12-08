use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::Foundation::{CloseHandle, HANDLE, MAX_PATH};
use windows::Win32::System::ProcessStatus::{EnumProcesses, GetModuleFileNameExW};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

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
pub fn is_running(exe_path: &PathBuf, running: &HashSet<PathBuf>) -> bool {
    // Normalize path for comparison
    let normalized = exe_path.to_string_lossy().to_lowercase();
    
    running.iter().any(|p| {
        p.to_string_lossy().to_lowercase() == normalized
    })
}
