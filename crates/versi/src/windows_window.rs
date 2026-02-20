#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowA, GetWindowTextW};

#[cfg(windows)]
const APP_TITLE_EXACT: &[u8] = b"Versi\0";

#[cfg(windows)]
const APP_TITLE_PREFIX: &str = "Versi";

#[cfg(windows)]
// SAFETY: callback signature is required by `EnumWindows`. `lparam` is always
// a pointer to writable `HWND` storage provided by `find_versi_window`.
unsafe extern "system" fn find_window_by_prefix(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out_ptr = lparam as *mut HWND;
    if out_ptr.is_null() {
        return TRUE;
    }

    // Query a bounded title buffer for this top-level window and look for the
    // stable application prefix, allowing dynamic suffixes like "Versi - Node ...".
    let mut title = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    if len <= 0 {
        return TRUE;
    }

    let title = String::from_utf16_lossy(&title[..len as usize]);
    if title == APP_TITLE_PREFIX || title.starts_with("Versi - ") {
        // SAFETY: `out_ptr` points to stack storage owned by the caller for
        // the entire `EnumWindows` call.
        unsafe { *out_ptr = hwnd };
        return 0;
    }

    TRUE
}

#[cfg(windows)]
pub(crate) fn find_versi_window() -> Option<HWND> {
    // Fast path for the static title used by loading/setup states.
    // SAFETY: class pointer is null and title pointer is a static
    // NUL-terminated string.
    let hwnd = unsafe { FindWindowA(ptr::null(), APP_TITLE_EXACT.as_ptr()) };
    if !hwnd.is_null() {
        return Some(hwnd);
    }

    // Fallback for dynamic titles (for example "Versi - Node v20.11.0").
    let mut found: HWND = ptr::null_mut();
    // SAFETY: callback and `lparam` both satisfy `EnumWindows` requirements.
    unsafe {
        EnumWindows(
            Some(find_window_by_prefix),
            &mut found as *mut HWND as LPARAM,
        );
    }
    if found.is_null() { None } else { Some(found) }
}
