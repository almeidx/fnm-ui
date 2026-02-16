pub(crate) fn set_update_badge(visible: bool) {
    use std::ptr;

    use log::debug;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, HBITMAP, HDC,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize,
    };
    use windows::Win32::UI::Shell::ITaskbarList3;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateIconIndirect, DestroyIcon, FindWindowA, HICON, ICONINFO,
    };
    use windows::core::{PCSTR, PCWSTR, s, w};

    struct GdiGuard {
        dc: Option<HDC>,
        color_bitmap: Option<HBITMAP>,
        mask_bitmap: Option<HBITMAP>,
        icon: Option<HICON>,
    }
    impl Drop for GdiGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(icon) = self.icon.take() {
                    let _ = DestroyIcon(icon);
                }
                if let Some(bm) = self.color_bitmap.take() {
                    let _ = DeleteObject(bm.into());
                }
                if let Some(bm) = self.mask_bitmap.take() {
                    let _ = DeleteObject(bm.into());
                }
                if let Some(dc) = self.dc.take() {
                    let _ = DeleteDC(dc);
                }
            }
        }
    }

    unsafe {
        let hwnd = match FindWindowA(PCSTR::null(), s!("Versi")) {
            Ok(h) if !h.is_invalid() => h,
            _ => {
                debug!("Could not find Versi window for badge");
                return;
            }
        };

        let com_initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();

        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let taskbar: ITaskbarList3 = CoCreateInstance(
                &windows::Win32::UI::Shell::TaskbarList,
                None,
                CLSCTX_INPROC_SERVER,
            )?;

            if !visible {
                taskbar.SetOverlayIcon(hwnd, HICON::default(), PCWSTR::null())?;
                return Ok(());
            }

            // Create a 16x16 red circle icon
            let size: i32 = 16;
            let mut pixels = vec![0u8; (size * size * 4) as usize];

            let center = size as f32 / 2.0;
            let radius = center - 1.0;

            for y in 0..size {
                for x in 0..size {
                    let dx = x as f32 - center + 0.5;
                    let dy = y as f32 - center + 0.5;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let offset = ((y * size + x) * 4) as usize;

                    if dist <= radius {
                        // BGRA format: red circle
                        pixels[offset] = 0x33; // B
                        pixels[offset + 1] = 0x33; // G
                        pixels[offset + 2] = 0xEE; // R
                        pixels[offset + 3] = 0xFF; // A
                    }
                }
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: size,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut guard = GdiGuard {
                dc: None,
                color_bitmap: None,
                mask_bitmap: None,
                icon: None,
            };

            let dc = CreateCompatibleDC(None);
            guard.dc = Some(dc);
            let mut bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
            guard.color_bitmap = Some(CreateDIBSection(
                Some(dc),
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                None,
                0,
            )?);
            ptr::copy_nonoverlapping(pixels.as_ptr(), bits_ptr as *mut u8, pixels.len());

            // Create mask bitmap (all zeros = fully opaque)
            let mask_bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: size,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut mask_bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
            guard.mask_bitmap = Some(CreateDIBSection(
                Some(dc),
                &mask_bmi,
                DIB_RGB_COLORS,
                &mut mask_bits_ptr,
                None,
                0,
            )?);
            ptr::write_bytes(mask_bits_ptr as *mut u8, 0, pixels.len());

            let icon_info = ICONINFO {
                fIcon: true.into(),
                xHotspot: 0,
                yHotspot: 0,
                hbmMask: guard.mask_bitmap.as_ref().copied().unwrap(),
                hbmColor: guard.color_bitmap.as_ref().copied().unwrap(),
            };

            guard.icon = Some(CreateIconIndirect(&icon_info)?);
            let result = taskbar.SetOverlayIcon(
                hwnd,
                guard.icon.as_ref().copied().unwrap(),
                w!("Update available"),
            );

            // Guard's Drop cleans up dc, color_bitmap, mask_bitmap, icon

            result?;
            Ok(())
        })();

        if com_initialized {
            CoUninitialize();
        }

        if let Err(e) = result {
            debug!("Failed to set update badge: {}", e);
        }
    }
}

pub(crate) fn set_dock_visible(_visible: bool) {}

pub(crate) fn is_wayland() -> bool {
    false
}

pub(crate) fn set_launch_at_login(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW,
        RegSetValueExW,
    };

    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "Versi\0".encode_utf16().collect();

    unsafe {
        let mut hkey = std::mem::zeroed();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if status != 0 {
            return Err(format!("RegOpenKeyExW failed: {status}").into());
        }

        let result = if enable {
            let exe = std::env::current_exe()?;
            let exe_wide: Vec<u16> = exe
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let byte_len = exe_wide.len() * 2;
            RegSetValueExW(
                hkey,
                value_name.as_ptr(),
                0,
                REG_SZ,
                exe_wide.as_ptr() as *const u8,
                byte_len as u32,
            )
        } else {
            RegDeleteValueW(hkey, value_name.as_ptr())
        };

        RegCloseKey(hkey);

        if result != 0 && !(result == 2 && !enable) {
            return Err(format!("Registry operation failed: {result}").into());
        }
    }

    Ok(())
}

pub(crate) fn reveal_in_file_manager(path: &std::path::Path) {
    use versi_core::HideWindow;
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.to_string_lossy()))
        .hide_window()
        .spawn();
}
