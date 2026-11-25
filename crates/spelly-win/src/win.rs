//! Win32 glue: low-level keyboard hook in, injected keystrokes out, tray icon.
//! All decisions live in `spelly_core::Typist`; this file only talks to Windows.

use std::cell::RefCell;
use spelly_core::{Edit, Key, Outcome, Pack, Runtime, Store};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;


/// Tagged on every key we inject, so our own hook ignores them.
const MARK: usize = 0x5350_454C; // "SPEL"
const WM_TRAY: u32 = WM_APP + 1;
const ID_PAUSE: usize = 1;
const ID_OPEN: usize = 5;
const ID_QUIT: usize = 3;
const ID_FOLDER: usize = 4;

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
    static APP: RefCell<(isize, String)> = const { RefCell::new((0, String::new())) };
    static LAST_FG: RefCell<isize> = const { RefCell::new(0) };
}

/// File name of the process that owns `hwnd`, e.g. `Code.exe`. Cached per window.
fn app_name(hwnd: HWND) -> String {
    let key = hwnd.0 as isize;
    if let Some(hit) = APP.with(|c| (c.borrow().0 == key).then(|| c.borrow().1.clone())) {
        return hit;
    }
    let mut name = String::new();
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                name = full.rsplit(['\\', '/']).next().unwrap_or("").to_string();
            }
            let _ = windows::Win32::Foundation::CloseHandle(h);
        }
    }
    APP.with(|c| *c.borrow_mut() = (key, name.clone()));
    name
}

/// `spelly-ui.exe`, installed next to us.
fn open_window() {
    let ui = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("spelly-ui.exe")));
    if let Some(ui) = ui.filter(|p| p.exists()) {
        let _ = std::process::Command::new(ui).spawn();
    }
}

pub fn run() {
    // One copy only: two hooks would both "fix" every word. The installer watches this name too.
    let _single = unsafe { CreateMutexW(None, false, w!("Local\\SpellyRunning")) };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return;
    }
    RUNTIME.with(|r| *r.borrow_mut() = Some(Runtime::new(Store::open_default())));
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let hwnd = create_tray_window(hinst.into());
        let _kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(hinst.into()), 0);
        let _ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), Some(hinst.into()), 0);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        remove_tray(hwnd);
    }
}

// ---------------------------------------------------------------- hooks

unsafe extern "system" fn keyboard_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let down = wp.0 as u32 == WM_KEYDOWN || wp.0 as u32 == WM_SYSKEYDOWN;
        if down && k.dwExtraInfo != MARK && handle_key(k) {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn mouse_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && matches!(wp.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN)
    {
        feed(Key::Reset);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

fn feed(key: Key) -> Outcome {
    let app = if matches!(key, Key::Reset) { String::new() } else { app_name(unsafe { GetForegroundWindow() }) };
    RUNTIME.with(|r| match r.borrow_mut().as_mut() {
        Some(rt) => rt.on_key(key, &app),
        None => Outcome::Pass,
    })
}

/// Returns true if the key was swallowed.
fn handle_key(k: &KBDLLHOOKSTRUCT) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        let changed = LAST_FG.with(|l| std::mem::replace(&mut *l.borrow_mut(), fg.0 as isize) != fg.0 as isize);
        if changed {
            feed(Key::Reset);
        }
        if in_password_field(fg) {
            feed(Key::Reset);
            return false;
        }
    }
    let Some(key) = translate(k) else { return false };
    match feed(key) {
        Outcome::Pass => false,
        Outcome::Swallow(edit) => {
            inject(&edit);
            true
        }
    }
}

// ---------------------------------------------------------------- key translation

fn is_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(vk.0 as i32) < 0 }
}

fn translate(k: &KBDLLHOOKSTRUCT) -> Option<Key> {
    let vk = VIRTUAL_KEY(k.vkCode as u16);
    let ctrl = is_down(VK_CONTROL);
    let alt = is_down(VK_MENU);
    let win = is_down(VK_LWIN) || is_down(VK_RWIN);

    match vk {
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU
        | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN | VK_CAPITAL | VK_NUMLOCK | VK_SCROLL => {
            return None
        }
        VK_BACK if !ctrl && !alt => return Some(Key::Backspace),
        VK_RETURN if !ctrl && !alt && !win => return Some(Key::Char('\n')),
        VK_TAB if !ctrl && !alt && !win => return Some(Key::Char('\t')),
        _ => {}
    }
    // Ctrl/Win/Alt shortcuts, navigation, delete, escape, F-keys: we lose track of the text.
    // Ctrl+Alt together is AltGr on many layouts and still types characters.
    let altgr = ctrl && alt;
    if win || (ctrl && !altgr) || (alt && !ctrl) || is_navigation(vk) {
        return Some(Key::Reset);
    }
    char_for(k).map(Key::Char)
}

fn is_navigation(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN | VK_HOME | VK_END | VK_PRIOR | VK_NEXT | VK_DELETE
            | VK_INSERT | VK_ESCAPE | VK_BACK
    ) || (VK_F1.0..=VK_F24.0).contains(&vk.0)
}

/// The character this key press produces on the current layout, if exactly one.
fn char_for(k: &KBDLLHOOKSTRUCT) -> Option<char> {
    unsafe {
        let mut state = [0u8; 256];
        for vk in [VK_SHIFT, VK_CONTROL, VK_MENU] {
            if is_down(vk) {
                state[vk.0 as usize] = 0x80;
            }
        }
        if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
            state[VK_CAPITAL.0 as usize] = 1;
        }
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        let layout = GetKeyboardLayout(tid);
        let mut buf = [0u16; 8];
        // flag 0x4: do not change the keyboard state (so dead keys still work for the app).
        let n = ToUnicodeEx(k.vkCode, k.scanCode, &state, &mut buf, 0x4, Some(layout));
        if n != 1 {
            return None; // dead key, no char, or a ligature: leave it alone
        }
        char::from_u32(buf[0] as u32).filter(|c| !c.is_control())
    }
}

/// True if the focused control is a password box.
unsafe fn in_password_field(fg: HWND) -> bool {
    if fg.0.is_null() {
        return false;
    }
    let tid = unsafe { GetWindowThreadProcessId(fg, None) };
    let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
    if unsafe { GetGUIThreadInfo(tid, &mut info) }.is_err() || info.hwndFocus.0.is_null() {
        return false;
    }
    const EM_GETPASSWORDCHAR: u32 = 0x00D2;
    let mut ch = 0usize;
    let ok = unsafe {
        SendMessageTimeoutW(
            info.hwndFocus,
            EM_GETPASSWORDCHAR,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            30,
            Some(&mut ch),
        )
    };
    ok.0 != 0 && ch != 0
}

// ---------------------------------------------------------------- injection

fn inject(edit: &Edit) {
    let mut inputs: Vec<INPUT> = vec![];
    fn tap(inputs: &mut Vec<INPUT>, vk: VIRTUAL_KEY) {
        inputs.push(key_input(vk, 0, KEYBD_EVENT_FLAGS(0)));
        inputs.push(key_input(vk, 0, KEYEVENTF_KEYUP));
    }
    for _ in 0..edit.backspaces {
        tap(&mut inputs, VK_BACK);
    }
    for c in edit.text.chars() {
        match c {
            '\n' => tap(&mut inputs, VK_RETURN),
            '\t' => tap(&mut inputs, VK_TAB),
            _ => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    inputs.push(key_input(VIRTUAL_KEY(0), *u, KEYEVENTF_UNICODE));
                    inputs.push(key_input(VIRTUAL_KEY(0), *u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
                }
            }
        }
    }
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
}

fn key_input(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: MARK },
        },
    }
}

// ---------------------------------------------------------------- tray

unsafe fn create_tray_window(hinst: windows::Win32::Foundation::HINSTANCE) -> HWND {
    let class = w!("SpellyTray");
    let wc = WNDCLASSW { lpfnWndProc: Some(tray_proc), hInstance: hinst, lpszClassName: class, ..Default::default() };
    unsafe { RegisterClassW(&wc) };
    let hwnd = unsafe {
        CreateWindowExW(WINDOW_EX_STYLE(0), class, w!("Spelly"), WINDOW_STYLE(0), 0, 0, 0, 0, None, None, Some(hinst), None)
    }
    .unwrap_or_default();
    let mut nid = tray_data(hwnd);
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    // Resource id 1 is the cat embedded by build.rs; fall back to the stock icon.
    nid.hIcon = unsafe { LoadIconW(Some(hinst), PCWSTR(1 as *const u16)) }
        .or_else(|_| unsafe { LoadIconW(None, IDI_APPLICATION) })
        .unwrap_or_default();
    for (i, u) in "Spelly".encode_utf16().enumerate() {
        nid.szTip[i] = u;
    }
    unsafe { let _ = Shell_NotifyIconW(NIM_ADD, &nid); }
    unsafe { SetTimer(Some(hwnd), 1, 1000, None) }; // once a second: pick up edits from the window
    hwnd
}

fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() }
}

unsafe fn remove_tray(hwnd: HWND) {
    unsafe { let _ = Shell_NotifyIconW(NIM_DELETE, &tray_data(hwnd)); }
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY if lp.0 as u32 == WM_RBUTTONUP || lp.0 as u32 == WM_LBUTTONUP => {
            unsafe { show_menu(hwnd) };
            LRESULT(0)
        }
        WM_TIMER => {
            RUNTIME.with(|r| {
                if let Some(rt) = r.borrow_mut().as_mut() {
                    rt.tick();
                }
            });
            LRESULT(0)
        }
        WM_COMMAND => {
            match wp.0 & 0xFFFF {
                ID_OPEN => open_window(),
                ID_PAUSE => RUNTIME.with(|r| {
                    if let Some(rt) = r.borrow_mut().as_mut() {
                        let now = !rt.settings().paused;
                        rt.set_paused(now);
                    }
                }),
                ID_FOLDER => {
                    let dir = Store::open_default().packs_dir();
                    Pack::seed_dir(&dir);
                    let _ = std::process::Command::new("explorer").arg(dir).spawn();
                }
                ID_QUIT => unsafe { PostQuitMessage(0) },
                _ => {}
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let paused = RUNTIME.with(|r| r.borrow().as_ref().is_some_and(|r| r.settings().paused));
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else { return };
    let pause_flags = if paused { MF_STRING | MF_CHECKED } else { MF_STRING };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, ID_OPEN, w!("Open Spelly"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, pause_flags, ID_PAUSE, w!("Pause"));
        let _ = AppendMenuW(menu, MF_STRING, ID_FOLDER, w!("Open dictionaries folder"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quit"));
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd); // required or the menu never closes
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), hwnd, None);
        let _ = DestroyMenu(menu);
    }
}
