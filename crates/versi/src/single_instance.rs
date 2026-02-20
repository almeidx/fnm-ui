#[derive(Debug)]
pub enum AcquireError {
    AlreadyRunning,
    Unavailable(String),
}

#[cfg(windows)]
mod windows_impl {
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::CreateMutexA;

    const MUTEX_NAME: &[u8] = b"Global\\VersiAppMutex\0";

    pub struct SingleInstance {
        handle: HANDLE,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Self, super::AcquireError> {
            unsafe {
                let handle = CreateMutexA(ptr::null(), 1, MUTEX_NAME.as_ptr());

                if handle.is_null() {
                    return Err(super::AcquireError::Unavailable(
                        "CreateMutexA returned a null handle".to_string(),
                    ));
                }

                let last_error = GetLastError();
                if last_error == ERROR_ALREADY_EXISTS {
                    CloseHandle(handle);
                    return Err(super::AcquireError::AlreadyRunning);
                }

                Ok(Self { handle })
            }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    pub fn bring_existing_window_to_front() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowA, SW_RESTORE, SetForegroundWindow, ShowWindow,
        };

        unsafe {
            let hwnd = FindWindowA(ptr::null(), b"Versi\0".as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
mod other_impl {
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;

    use fs2::FileExt;
    use versi_platform::AppPaths;

    fn lock_file_path() -> Result<PathBuf, super::AcquireError> {
        let paths = AppPaths::new().map_err(super::AcquireError::Unavailable)?;
        paths.ensure_dirs().map_err(|error| {
            super::AcquireError::Unavailable(format!(
                "failed to create app directories: {error}"
            ))
        })?;
        Ok(paths.data_dir.join("instance.lock"))
    }

    pub struct SingleInstance {
        _file: File,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Self, super::AcquireError> {
            let lock_file_path = lock_file_path()?;
            let mut lock_file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(lock_file_path)
                .map_err(|error| {
                    super::AcquireError::Unavailable(format!(
                        "failed to open instance lock file: {error}"
                    ))
                })?;

            match lock_file.try_lock_exclusive() {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    return Err(super::AcquireError::AlreadyRunning);
                }
                Err(error) => {
                    return Err(super::AcquireError::Unavailable(format!(
                        "failed to acquire instance lock: {error}"
                    )));
                }
            }

            lock_file
                .set_len(0)
                .and_then(|()| lock_file.seek(SeekFrom::Start(0)).map(|_| ()))
                .and_then(|()| writeln!(lock_file, "{}", std::process::id()))
                .map_err(|error| {
                    super::AcquireError::Unavailable(format!(
                        "failed to write instance lock metadata: {error}"
                    ))
                })?;

            Ok(Self { _file: lock_file })
        }
    }

    pub fn bring_existing_window_to_front() {}
}

#[cfg(not(windows))]
pub use other_impl::{SingleInstance, bring_existing_window_to_front};
#[cfg(windows)]
pub use windows_impl::{SingleInstance, bring_existing_window_to_front};

#[cfg(all(test, not(windows)))]
mod tests {
    use super::{SingleInstance, bring_existing_window_to_front};

    #[test]
    fn non_windows_acquire_returns_instance() {
        assert!(SingleInstance::acquire().is_ok());
    }

    #[test]
    fn non_windows_bring_existing_window_to_front_is_noop() {
        bring_existing_window_to_front();
    }
}
