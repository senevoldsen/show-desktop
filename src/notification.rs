use std::mem::size_of;

use windows::{
    core::{w, Error, Result, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

// Seems to be missing. See https://github.com/microsoft/win32metadata/issues/1765
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;

const TIP: PCWSTR = w!("Window Hider");
const WND_CLASSNAME: PCWSTR = w!("WinHiderTrayCls");

const NOTIFY_ICON_UID: u32 = 10;
const NOTIFICATION_MESSAGE_ID: u32 = WM_USER + 20;
const IDM_EXIT: usize = 100;

pub fn loword(l: usize) -> usize {
    l & 0xffff
}

#[allow(non_snake_case)]
unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    uMsg: u32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    // See WM_CREATE comment.
    static mut MSG_TASKBAR_CREATED: u32 = 0;

    match uMsg {
        WM_CREATE => {
            // Handle the case when explorer is restarted.
            // See: https://stackoverflow.com/a/32045617/6130089
            MSG_TASKBAR_CREATED = RegisterWindowMessageW(w!("TaskbarCreated"));
            if add_notification_icon(hwnd).is_err() {
                PostQuitMessage(1);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: NOTIFY_ICON_UID,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            PostQuitMessage(0);
            LRESULT(0)
        }
        NOTIFICATION_MESSAGE_ID => {
            match loword(lParam.0 as usize) as u32 {
                WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT => {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let menu = CreatePopupMenu().unwrap();
                    let _ = InsertMenuW(menu, 0, MF_BYPOSITION | MF_STRING, IDM_EXIT, w!("Exit"));
                    let _ = SetForegroundWindow(hwnd);
                    let _ = TrackPopupMenu(
                        menu,
                        TPM_LEFTALIGN | TPM_LEFTBUTTON | TPM_BOTTOMALIGN,
                        pt.x,
                        pt.y,
                        0,
                        hwnd,
                        Default::default(),
                    );
                }
                _ => (),
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            if lParam.0 == 0 && loword(wParam.0) == IDM_EXIT {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, uMsg, wParam, lParam)
            }
        }
        _ => {
            if uMsg == MSG_TASKBAR_CREATED {
                if add_notification_icon(hwnd).is_err() {
                    PostQuitMessage(1);
                }
            }
            DefWindowProcW(hwnd, uMsg, wParam, lParam)
        }
    }
}

pub unsafe fn create_window() -> Result<HWND> {
    let module_handle = GetModuleHandleW(PCWSTR::null()).unwrap();
    let wnd_class = WNDCLASSEXW {
        lpszClassName: WND_CLASSNAME,
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(tray_window_proc),
        hIcon: HICON(
            LoadImageW(
                module_handle,
                IDI_APPLICATION,
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTCOLOR,
            )
            .unwrap()
            .0,
        ),
        hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap(),
        hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
        ..Default::default()
    };

    if RegisterClassExW(&wnd_class) == 0 {
        return Err(Error::from_win32());
    }

    CreateWindowExW(
        WS_EX_NOACTIVATE,
        WND_CLASSNAME,
        TIP,
        WS_POPUP,
        0,
        0,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        HWND::default(),
        HMENU::default(),
        HINSTANCE::default(),
        None,
    )
}

#[allow(non_snake_case)]
pub unsafe fn add_notification_icon(hwnd: HWND) -> Result<()> {
    let module_handle = GetModuleHandleW(PCWSTR::null()).unwrap();

    let mut tip = [0u16; 128];
    tip[0..TIP.len()].copy_from_slice(TIP.as_wide());

    let notify_data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: NOTIFY_ICON_UID,
        uFlags: NIF_MESSAGE | NIF_TIP | NIF_ICON | NIF_SHOWTIP,
        Anonymous: NOTIFYICONDATAW_0 {
            uVersion: NOTIFYICON_VERSION_4,
        },
        uCallbackMessage: NOTIFICATION_MESSAGE_ID,
        hIcon: HICON(
            LoadImageW(
                module_handle,
                IDI_APPLICATION,
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTCOLOR,
            )
            .unwrap()
            .0,
        ),
        szTip: tip,
        ..Default::default()
    };

    let result = Shell_NotifyIconW(NIM_ADD, &notify_data).as_bool();
    if !result {
        Err(Error::empty())
    } else {
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &notify_data);
        Ok(())
    }
}
