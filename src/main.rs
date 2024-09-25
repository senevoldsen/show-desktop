#![windows_subsystem = "windows"]

use std::ptr;

use window::hide_windows_on_active_monitor;
use windows::{
    core::{Free, Result, PCWSTR},
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, VIRTUAL_KEY, VK_D, VK_LWIN},
            WindowsAndMessaging::*,
        },
    },
};

mod notification;
mod window;

fn main() -> Result<()> {
    unsafe {
        let handle: HMODULE = GetModuleHandleW(PCWSTR::null()).unwrap();
        let mut keyboard_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keyboard_event), handle, 0)?;
        let _ = notification::create_window();
        message_loop(&mut keyboard_hook);
    }
    Ok(())
}

#[allow(non_snake_case)]
unsafe extern "system" fn ll_keyboard_event(
    code: i32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    // Check whether we must return without processing.
    if code < 0 {
        return CallNextHookEx(HHOOK(ptr::null_mut()), code, wParam, lParam);
    }
    // Default hotkey is: LWIN + D
    // In that order.
    if wParam.0 as u32 == WM_KEYDOWN {
        let event: &KBDLLHOOKSTRUCT = &*(lParam.0 as *const _);
        if VIRTUAL_KEY(event.vkCode as u16) == VK_D {
            let is_lwin_down = GetKeyState(VK_LWIN.0 as i32) < 0;
            if is_lwin_down {
                let _ = hide_windows_on_active_monitor();
                // Return that we processed in the LL event.
                return LRESULT(-1);
            }
        }
    }
    CallNextHookEx(HHOOK(ptr::null_mut()), code, wParam, lParam)
}

pub unsafe fn message_loop(hook: &mut HHOOK) {
    let mut msg: MSG = MSG::default();
    loop {
        let no_error_or_quit: bool = GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool();
        if !no_error_or_quit {
            hook.free();
            hook.0 = ptr::null_mut();
            break;
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
