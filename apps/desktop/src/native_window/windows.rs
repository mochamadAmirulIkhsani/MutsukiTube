use std::ptr;

use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, IsWindow, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER,
        SetWindowPos, ShowWindow, WS_CHILD, WS_CLIPSIBLINGS,
    },
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub struct Win32VideoSurface {
    hwnd: HWND,
}

impl Win32VideoSurface {
    pub fn new(parent: HWND) -> Result<Self, String> {
        if parent.is_null() {
            return Err("Parent HWND is null".into());
        }

        let class = wide("STATIC");
        let title = wide("MutsukiTube Video Surface");

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_CLIPSIBLINGS,
                0,
                0,
                320,
                180,
                parent,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };

        if hwnd.is_null() {
            return Err(format!(
                "CreateWindowExW failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(Self { hwnd })
    }

    #[allow(dead_code)]
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
    }

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn set_geometry(&self, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
        let ok = unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                width.max(1),
                height.max(1),
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
        };

        if ok == 0 {
            return Err(format!(
                "SetWindowPos failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        unsafe { IsWindow(self.hwnd) != 0 }
    }
}

impl Drop for Win32VideoSurface {
    fn drop(&mut self) {
        if !self.hwnd.is_null() && self.is_valid() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}
