//! macOS glue: a CGEventTap in, injected CGEvents out, a menu bar item.
//! All decisions live in `spelly_core::Typist`; this file only talks to macOS.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::mach_port::CFMachPort;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_foundation::string::CFString;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, CallbackResult, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use spelly_core::{Edit, Key, Outcome, Runtime, Store};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

/// Tagged on every event we inject, so our own tap ignores them.
const MARK: i64 = 0x5350_454C; // "SPEL"

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
    static APP: RefCell<(Instant, String)> = RefCell::new((Instant::now() - Duration::from_secs(10), String::new()));
    static PORT: RefCell<Option<CFMachPort>> = const { RefCell::new(None) };
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
    fn CGEventTapEnable(tap: core_foundation::mach_port::CFMachPortRef, enable: bool);
    fn CGEventKeyboardGetUnicodeString(
        event: core_graphics::sys::CGEventRef,
        max_len: libc_ulong,
        actual_len: *mut libc_ulong,
        buf: *mut u16,
    );
}
#[allow(non_camel_case_types)]
type libc_ulong = std::os::raw::c_ulong;

enum UserEvent {
    Menu(tray_icon::menu::MenuId),
}

/// Name of the focused app, cached for a moment (asking the OS on every key is wasteful).
fn frontmost_app() -> String {
    APP.with(|c| {
        let mut c = c.borrow_mut();
        if c.0.elapsed() > Duration::from_millis(400) {
            use objc2_app_kit::NSWorkspace;
            let name = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .and_then(|a| a.localizedName())
                .map(|n| n.to_string());
            *c = (Instant::now(), name.unwrap_or_default());
        }
        c.1.clone()
    })
}

/// Open the window. In the shipped app that is the helper app inside our bundle (macOS keeps
/// it to one instance and just raises it); in development it is `spelly-ui` next to this binary.
fn open_window() {
    let Some(exe) = std::env::current_exe().ok() else { return };
    let helper = exe.parent().and_then(|d| d.parent()).map(|c| c.join("Helpers/Spelly Window.app"));
    if let Some(app) = helper.filter(|p| p.exists()) {
        let _ = std::process::Command::new("open").arg(app).spawn();
    } else if let Some(ui) = exe.parent().map(|d| d.join("spelly-ui")).filter(|p| p.exists()) {
        let _ = std::process::Command::new(ui).spawn();
    }
}

fn trusted(prompt: bool) -> bool {
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let opts = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::from(prompt))]);
    unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef()) }
}

pub fn run() {
    RUNTIME.with(|r| *r.borrow_mut() = Some(Runtime::new(Store::open_default())));
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory); // menu bar only, no Dock icon
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let _ = proxy.send_event(UserEvent::Menu(e.id));
    }));

    let open = MenuItem::new("Open Spelly", true, None);
    let pause = CheckMenuItem::new("Pause", true, false, None);
    let folder = MenuItem::new("Open dictionaries folder", true, None);
    let quit = MenuItem::new("Quit Spelly", true, None);
    let mut tray = None;
    let mut tap: Option<CGEventTap<'static>> = None;
    let mut asked = false;
    // Signing in starts us with --background: tray only, no window.
    let background = std::env::args().any(|a| a == "--background");

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                let menu = Menu::new();
                let _ = menu.append_items(&[&open, &PredefinedMenuItem::separator(), &pause, &folder, &PredefinedMenuItem::separator(), &quit]);
                tray = TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_icon(cat_icon())
                    .with_tooltip("Spelly")
                    .build()
                    .ok();
                if !background {
                    open_window();
                }
            }
            // Clicking Spelly in the Dock / Launchpad / Finder while it runs: show the window.
            Event::Reopen { .. } => open_window(),
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                // Once a second: pick up edits made in the window or by hand.
                RUNTIME.with(|r| {
                    if let Some(rt) = r.borrow_mut().as_mut() {
                        if rt.tick() {
                            pause.set_checked(rt.settings().paused);
                        }
                    }
                });
            }
            Event::UserEvent(UserEvent::Menu(id)) => {
                if id == *open.id() {
                    open_window();
                } else if id == *pause.id() {
                    let on = pause.is_checked();
                    RUNTIME.with(|r| {
                        if let Some(rt) = r.borrow_mut().as_mut() {
                            rt.set_paused(on);
                        }
                    });
                } else if id == *folder.id() {
                    let dir = Store::open_default().packs_dir();
                    spelly_core::Pack::seed_dir(&dir);
                    let _ = std::process::Command::new("open").arg(dir).spawn();
                } else if id == *quit.id() {
                    drop(tray.take());
                    *flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
        // Until macOS lets us in (Accessibility permission), keep asking nicely.
        if tap.is_none() && *flow != ControlFlow::Exit {
            if trusted(!asked) {
                tap = install_tap();
            }
            asked = true;
        }
        if *flow != ControlFlow::Exit {
            *flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));
        }
    });
}

fn install_tap() -> Option<CGEventTap<'static>> {
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![
            CGEventType::KeyDown,
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
            CGEventType::OtherMouseDown,
        ],
        |_proxy, ty, ev| on_event(ty, ev),
    )
    .ok()?;
    let source = tap.mach_port().create_runloop_source(0).ok()?;
    CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
    tap.enable();
    PORT.with(|p| *p.borrow_mut() = Some(tap.mach_port().clone()));
    Some(tap)
}

// ---------------------------------------------------------------- events

fn on_event(ty: CGEventType, ev: &CGEvent) -> CallbackResult {
    match ty {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            PORT.with(|p| {
                if let Some(port) = p.borrow().as_ref() {
                    unsafe { CGEventTapEnable(port.as_concrete_TypeRef(), true) };
                }
            });
            CallbackResult::Keep
        }
        CGEventType::LeftMouseDown | CGEventType::RightMouseDown | CGEventType::OtherMouseDown => {
            feed(Key::Reset);
            CallbackResult::Keep
        }
        CGEventType::KeyDown => {
            if ev.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA) == MARK {
                return CallbackResult::Keep;
            }
            let Some(key) = translate(ev) else { return CallbackResult::Keep };
            match feed(key) {
                Outcome::Pass => CallbackResult::Keep,
                Outcome::Swallow(edit) => {
                    inject(&edit);
                    CallbackResult::Drop
                }
            }
        }
        _ => CallbackResult::Keep,
    }
}

fn feed(key: Key) -> Outcome {
    let app = if matches!(key, Key::Reset) { String::new() } else { frontmost_app() };
    RUNTIME.with(|r| match r.borrow_mut().as_mut() {
        Some(rt) => rt.on_key(key, &app),
        None => Outcome::Pass,
    })
}

const K_DELETE: i64 = 51;
const K_RETURN: [i64; 2] = [36, 76];
const K_TAB: i64 = 48;
/// Arrows, Home/End, PgUp/PgDn, forward delete, Esc, Help, F1..F20.
const K_NAVIGATION: [i64; 29] = [
    123, 124, 125, 126, 115, 119, 116, 121, 117, 53, 114, 122, 120, 99, 118, 96, 97, 98, 100, 101,
    109, 103, 111, 105, 107, 113, 106, 64, 79,
];

fn translate(ev: &CGEvent) -> Option<Key> {
    let code = ev.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
    let flags = ev.get_flags();
    let shortcut = flags.contains(CGEventFlags::CGEventFlagCommand) || flags.contains(CGEventFlags::CGEventFlagControl);
    let option = flags.contains(CGEventFlags::CGEventFlagAlternate);

    if shortcut || K_NAVIGATION.contains(&code) || code == 80 || code == 90 {
        return Some(Key::Reset);
    }
    if code == K_DELETE {
        return Some(if option { Key::Reset } else { Key::Backspace }); // Option+Delete eats a word
    }
    if K_RETURN.contains(&code) {
        return Some(Key::Char('\n'));
    }
    if code == K_TAB {
        return Some(Key::Char('\t'));
    }
    let mut buf = [0u16; 8];
    let mut len: libc_ulong = 0;
    unsafe { CGEventKeyboardGetUnicodeString(foreign_types::ForeignType::as_ptr(ev) as _, 8, &mut len, buf.as_mut_ptr()) };
    if len != 1 {
        return None; // dead key, no text, or several chars: leave it alone
    }
    char::from_u32(buf[0] as u32).filter(|c| !c.is_control() && !('\u{F700}'..='\u{F8FF}').contains(c)).map(Key::Char)
}

// ---------------------------------------------------------------- injection

fn inject(edit: &Edit) {
    let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
    let post = |code: u16, text: Option<&str>| {
        for down in [true, false] {
            if let Ok(e) = CGEvent::new_keyboard_event(src.clone(), code, down) {
                e.set_flags(CGEventFlags::empty());
                e.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, MARK);
                if let Some(t) = text {
                    e.set_string(t);
                }
                e.post(CGEventTapLocation::HID);
            }
        }
    };
    for _ in 0..edit.backspaces {
        post(K_DELETE as u16, None);
    }
    let mut run = String::new();
    let flush = |run: &mut String| {
        if !run.is_empty() {
            post(0, Some(run));
            run.clear();
        }
    };
    for c in edit.text.chars() {
        match c {
            '\n' => {
                flush(&mut run);
                post(K_RETURN[0] as u16, None);
            }
            '\t' => {
                flush(&mut run);
                post(K_TAB as u16, None);
            }
            _ => run.push(c),
        }
    }
    flush(&mut run);
}

// ---------------------------------------------------------------- icon

/// The kitten, pre-rendered by packaging/make_icon.py (8-byte w,h header + RGBA).
fn cat_icon() -> Icon {
    const RAW: &[u8] = include_bytes!("../../../assets/tray.rgba");
    let w = u32::from_le_bytes(RAW[0..4].try_into().unwrap());
    let h = u32::from_le_bytes(RAW[4..8].try_into().unwrap());
    Icon::from_rgba(RAW[8..].to_vec(), w, h).expect("tray icon")
}
