use std::mem::size_of;

use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, RECT},
        Graphics::{
            Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
            Gdi::*,
        },
        UI::WindowsAndMessaging::*,
    },
};

struct WindowCollection {
    monitor: HMONITOR,
    windows: Vec<HWND>,
}

pub unsafe fn hide_windows_on_active_monitor() -> Result<()> {
    let foreground_window: HWND = GetForegroundWindow();
    let monitor = MonitorFromWindow(foreground_window, MONITOR_DEFAULTTONEAREST);
    if foreground_window.is_invalid() || monitor.is_invalid() {
        return Err(Error::empty());
    }

    let mut window_collection = WindowCollection {
        monitor,
        windows: Vec::new(),
    };
    EnumWindows(
        Some(collect_windows_to_hide),
        LPARAM(&mut window_collection as *mut _ as isize),
    )?;

    for window in window_collection.windows {
        let _ = ShowWindow(window, SW_SHOWMINNOACTIVE);
    }

    Ok(())
}

#[allow(non_snake_case)]
unsafe extern "system" fn collect_windows_to_hide(hwnd: HWND, lParam: LPARAM) -> BOOL {
    unsafe {
        let window_collection = &mut *(lParam.0 as *mut WindowCollection);
        match window_should_hide(hwnd, window_collection.monitor) {
            Ok(hide) if hide => {
                window_collection.windows.push(hwnd);
            }
            _ => {
                // Silently drop
            }
        }
    };
    true.into() // continue iterating
}

fn window_should_hide(window: HWND, monitor: HMONITOR) -> Result<bool> {
    const NO_HIDE: Result<bool> = Ok(false);

    // Is the window predominantly on the monitor to hide on?
    let window_monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };

    if monitor != window_monitor {
        return NO_HIDE;
    }

    // Ignore windows with no title.
    let window_text: String = get_window_title(window)?;
    if window_text.is_empty() {
        return NO_HIDE;
    }

    // Ignore hidden and child windows.
    let info: WINDOWINFO = get_window_info(window)?;
    if info.dwStyle.contains(WS_CHILD) || !info.dwStyle.contains(WS_VISIBLE) {
        return NO_HIDE;
    }

    // Ignore Iconic and owned windows that are not WX_EX_APPWINDOW.
    unsafe {
        let window_owner = GetWindow(window, GW_OWNER).unwrap_or(HWND::default());
        let ex_style = GetWindowLongW(window, GWL_EXSTYLE);
        if IsIconic(window).as_bool()
            || (!window_owner.is_invalid()
                && !WINDOW_EX_STYLE(ex_style as u32).contains(WS_EX_APPWINDOW))
        {
            return NO_HIDE;
        }
    }

    // Ignore cloaked windows.
    if is_window_cloaked(window)? {
        return NO_HIDE;
    }

    // Now check if even in view
    unsafe {
        let monitor_info = get_monitor_info(monitor)?;
        let mut overlap = RECT::default();
        if !IntersectRect(&mut overlap, &monitor_info.rcWork, &info.rcWindow).as_bool() {
            return NO_HIDE;
        }
    }

    // Ignore certain classnames
    const IGNORE_CLASSES: &[&str] = &[
        "Progman",
        "Button",
        "ApplicationFrameWindow",
        "Windows.UI.Core.CoreWindow",
    ];
    if let Ok(classname) = get_window_classname(window) {
        if IGNORE_CLASSES.iter().any(|&c| c == classname) {
            return NO_HIDE;
        }
    }

    Ok(true)
}

pub fn is_window_cloaked(window: HWND) -> Result<bool> {
    let mut cloaked: u32 = 0;
    unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            size_of::<u32>() as u32,
        )?;
    }
    Ok(cloaked != 0)
}

pub fn get_monitor_info(monitor: HMONITOR) -> Result<MONITORINFO> {
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            Ok(monitor_info)
        } else {
            Err(windows::core::Error::empty())
        }
    }
}

pub fn get_window_info(window: HWND) -> Result<WINDOWINFO> {
    unsafe {
        let mut info = WINDOWINFO {
            cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
            ..Default::default()
        };
        GetWindowInfo(window, &mut info)?;
        Ok(info)
    }
}

fn get_window_title(window: HWND) -> Result<String> {
    let mut title: [u16; 512] = [0; 512];
    let len = unsafe { GetWindowTextW(window, &mut title) };
    if len == 0 {
        Err(windows::core::Error::from_win32())
    } else {
        Ok(String::from_utf16_lossy(&title[..len as usize]))
    }
}

fn get_window_classname(window: HWND) -> Result<String> {
    let mut classname = [0u16; 256];
    let len = unsafe { GetClassNameW(window, &mut classname) };
    if len == 0 {
        Err(windows::core::Error::from_win32())
    } else {
        Ok(String::from_utf16_lossy(&classname[..len as usize]))
    }
}

#[allow(dead_code)]
pub unsafe fn print_window_name(window: HWND) {
    let mut text: [u16; 512] = [0; 512];
    let len = GetWindowTextW(window, &mut text);
    let text = String::from_utf16_lossy(&text[..len as usize]);

    let mut info = WINDOWINFO {
        cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
        ..Default::default()
    };
    GetWindowInfo(window, &mut info).unwrap();

    if !text.is_empty() && info.dwStyle.contains(WS_VISIBLE) {
        println!("{} ({}, {})", text, info.rcWindow.left, info.rcWindow.top);
    }
}
