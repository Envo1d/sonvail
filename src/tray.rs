//! Windows notification-area integration. Native callbacks only enqueue commands;
//! GPUI handles them after returning from the Windows message callback.
use std::{cell::Cell, rc::Rc};

use anyhow::{Context, Result, bail};
use gpui::{App, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::SetFocus, Shell::*, WindowsAndMessaging::*},
    },
    core::{PCWSTR, w},
};

const CALLBACK: u32 = WM_APP + 71;
const SUBCLASS: usize = 0x534f4e56;
const OPEN: usize = 1;
const EXIT: usize = 2;
// NIN_KEYSELECT is a C macro rather than an exported windows-rs constant.
const KEY_SELECT: u32 = NIN_SELECT | NINF_KEY;

enum Command {
    Open,
    Exit,
}

pub struct Tray {
    data: NOTIFYICONDATAW,
    menu: HMENU,
    taskbar_created: u32,
    commands: async_channel::Sender<Command>,
    attached: Cell<bool>,
}

pub fn install(window: &Window, cx: &mut App) -> Result<Rc<Tray>> {
    let native_handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("could not get native window: {error}"))?;
    let RawWindowHandle::Win32(handle) = native_handle.as_raw() else {
        bail!("expected a Windows window handle");
    };
    let (sender, receiver) = async_channel::unbounded();
    // All handles and the subclass are created, used and released on the UI thread.
    let tray = unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        #[expect(
            clippy::manual_dangling_ptr,
            reason = "Win32 MAKEINTRESOURCEW encodes resource ID 1 as a pointer, never dereferenced"
        )]
        let icon_resource = PCWSTR(1usize as *const u16);
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: HWND(handle.hwnd.get() as *mut _),
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: CALLBACK,
            // Shared resource icon: Windows owns its lifetime.
            hIcon: LoadIconW(Some(instance), icon_resource)?,
            ..Default::default()
        };
        for (target, character) in data.szTip.iter_mut().zip("Sonvail".encode_utf16()) {
            *target = character;
        }
        let menu = CreatePopupMenu()?;
        let tray = Rc::new(Tray {
            data,
            menu,
            taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            commands: sender,
            attached: Cell::new(false),
        });
        AppendMenuW(menu, MF_STRING, OPEN, w!("Open Sonvail"))?;
        AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(menu, MF_STRING, EXIT, w!("Exit"))?;
        if tray.taskbar_created == 0 {
            bail!("could not register Explorer restart notification");
        }
        SetWindowSubclass(
            tray.data.hWnd,
            Some(callback),
            SUBCLASS,
            Rc::as_ptr(&tray) as usize,
        )
        .ok()
        .context("could not install tray callback")?;
        tray.attached.set(true);
        tray.add()?;
        tray
    };

    let shutdown_tray = tray.clone();
    cx.on_app_quit(move |_| {
        shutdown_tray.shutdown();
        std::future::ready(())
    })
    .detach();
    let hwnd = tray.data.hWnd;
    cx.spawn(async move |cx| {
        while let Ok(command) = receiver.recv().await {
            match command {
                Command::Open => {
                    restore(hwnd);
                }
                Command::Exit => {
                    let _ = cx.update(|cx| cx.quit());
                    break;
                }
            }
        }
    })
    .detach();
    Ok(tray)
}

fn restore(hwnd: HWND) {
    // Run on the UI executor without an App/Window borrow: activation and focus
    // synchronously dispatch native events back into GPUI. Its 0.2.2 activate()
    // also logs a null previous active/focus HWND as an error with HRESULT(0).
    unsafe {
        let mode = if IsIconic(hwnd).as_bool() {
            SW_RESTORE
        } else {
            SW_SHOW
        };
        // Show synchronously here so the window is visible before focusing it.
        // ShowWindow returns its previous visibility, not a success flag.
        let _ = ShowWindow(hwnd, mode);
        if !IsWindowVisible(hwnd).as_bool() {
            tracing::warn!("Could not restore Sonvail window from the system tray");
            return;
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            tracing::debug!("Windows did not grant foreground activation to Sonvail");
        }
        // SetFocus returns the PREVIOUS focus HWND, which can legitimately be
        // null. windows-rs represents that as Err even when GetLastError is zero.
        if let Err(error) = SetFocus(Some(hwnd))
            && error.code().is_err()
        {
            tracing::warn!(%error, "Could not focus the restored Sonvail window");
        }
        tracing::info!("Restored Sonvail from the system tray");
    }
}

impl Tray {
    fn add(&self) -> Result<()> {
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.data)
                .ok()
                .context("could not add Sonvail notification icon")?;
            let mut version = self.data;
            version.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            if !Shell_NotifyIconW(NIM_SETVERSION, &version).as_bool() {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
                bail!("could not set notification icon version");
            }
        }
        Ok(())
    }

    pub fn hide(&self) -> bool {
        unsafe {
            // Verify that a restore affordance exists before hiding the only window.
            if !Shell_NotifyIconW(NIM_MODIFY, &self.data).as_bool()
                && let Err(error) = self.add()
            {
                tracing::error!(%error, "Cannot hide window without a tray icon");
                return false;
            }
            tracing::info!("Hiding Sonvail in the system tray");
            if !ShowWindowAsync(self.data.hWnd, SW_HIDE).as_bool() {
                tracing::error!("Could not hide Sonvail window");
                return false;
            }
        }
        true
    }

    fn shutdown(&self) {
        if self.attached.replace(false) {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
                let _ = RemoveWindowSubclass(self.data.hWnd, Some(callback), SUBCLASS);
            }
            self.commands.close();
        }
    }

    unsafe fn show_menu(&self) {
        unsafe {
            let mut point = POINT::default();
            if GetCursorPos(&mut point).is_err() {
                return;
            }
            let _ = SetForegroundWindow(self.data.hWnd);
            let selected = TrackPopupMenu(
                self.menu,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                None,
                self.data.hWnd,
                None,
            );
            let _ = PostMessageW(Some(self.data.hWnd), WM_NULL, WPARAM(0), LPARAM(0));
            let command = match selected.0 as usize {
                OPEN => Command::Open,
                EXIT => Command::Exit,
                _ => {
                    let _ = Shell_NotifyIconW(NIM_SETFOCUS, &self.data);
                    return;
                }
            };
            let _ = self.commands.try_send(command);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.shutdown();
        unsafe {
            let _ = DestroyMenu(self.menu);
        }
    }
}

unsafe extern "system" fn callback(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    state: usize,
) -> LRESULT {
    // The Rc allocation stays alive until shutdown removes this subclass.
    let tray = unsafe { &*(state as *const Tray) };
    if message == CALLBACK {
        match (lparam.0 as u32) & 0xffff {
            NIN_SELECT | KEY_SELECT => {
                let _ = tray.commands.try_send(Command::Open);
            }
            WM_CONTEXTMENU => unsafe { tray.show_menu() },
            _ => {}
        }
        return LRESULT(0);
    }
    if message == tray.taskbar_created {
        if let Err(error) = tray.add() {
            tracing::error!(%error, "Could not restore tray after Explorer restart");
            let _ = tray.commands.try_send(Command::Open);
        }
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}
