//! macOS glue: a CGEventTap in, injected CGEvents out, a menu bar item.
//! All decisions live in `spelly_core::Typist`; this file only talks to macOS.

use std::cell::RefCell;
use std::path::PathBuf;
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
use harper_core::Dialect;
use spelly_core::{Edit, Key, Outcome, Pack, Spelly, Typist};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

/// Tagged on every event we inject, so our own tap ignores them.
const MARK: i64 = 0x5350_454C; // "SPEL"

thread_local! {
    static TYPIST: RefCell<Option<Typist>> = const { RefCell::new(None) };
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

/// `~/Library/Application Support/Spelly/dictionaries`
fn dict_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Library/Application Support/Spelly/dictionaries")
}

fn build() -> Spelly {
    Spelly::new(&Pack::load_dir(&dict_dir()), Dialect::Indian)
}

fn trusted(prompt: bool) -> bool {
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let opts = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::from(prompt))]);
    unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef()) }
}

pub fn run() {
    TYPIST.with(|t| *t.borrow_mut() = Some(Typist::new(build())));
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory); // menu bar only, no Dock icon
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let _ = proxy.send_event(UserEvent::Menu(e.id));
    }));

    let pause = CheckMenuItem::new("Pause", true, false, None);
    let folder = MenuItem::new("Open dictionaries folder", true, None);
    let reload = MenuItem::new("Reload dictionaries", true, None);
    let quit = MenuItem::new("Quit Spelly", true, None);
    let mut tray = None;
    let mut tap: Option<CGEventTap<'static>> = None;
    let mut asked = false;

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                let menu = Menu::new();
                let _ = menu.append_items(&[&pause, &folder, &reload, &PredefinedMenuItem::separator(), &quit]);
                tray = TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_icon(cat_icon())
                    .with_tooltip("Spelly")
                    .build()
                    .ok();
            }
            Event::UserEvent(UserEvent::Menu(id)) => {
                if id == *pause.id() {
                    TYPIST.with(|t| {
                        if let Some(t) = t.borrow_mut().as_mut() {
                            t.paused = pause.is_checked();
                        }
                    });
                } else if id == *reload.id() {
                    let fresh = build();
                    TYPIST.with(|t| {
                        if let Some(t) = t.borrow_mut().as_mut() {
                            t.reload(fresh);
                        }
                    });
                } else if id == *folder.id() {
                    Pack::seed_dir(&dict_dir());
                    let _ = std::process::Command::new("open").arg(dict_dir()).spawn();
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
            if tap.is_none() {
                *flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(2));
            }
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
    TYPIST.with(|t| match t.borrow_mut().as_mut() {
        Some(t) => t.on_key(key),
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

/// A small pink cat face, drawn in code so there is no asset to lose.
fn cat_icon() -> Icon {
    const N: i32 = 44;
    let (pink, dark) = ([244u8, 114, 182, 255], [131u8, 24, 67, 255]);
    let mut px = vec![0u8; (N * N * 4) as usize];
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if (0..N).contains(&x) && (0..N).contains(&y) {
            let i = ((y * N + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    };
    let inside_tri = |x: f32, y: f32, a: (f32, f32), b: (f32, f32), c: (f32, f32)| {
        let s = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1);
        let (d1, d2, d3) = (s((x, y), a, b), s((x, y), b, c), s((x, y), c, a));
        !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
    };
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let face = ((fx - 22.0) / 17.0).powi(2) + ((fy - 25.0) / 14.0).powi(2) <= 1.0;
            let ears = inside_tri(fx, fy, (6.0, 22.0), (7.0, 4.0), (21.0, 13.0))
                || inside_tri(fx, fy, (38.0, 22.0), (37.0, 4.0), (23.0, 13.0));
            if face || ears {
                put(x, y, pink);
            }
        }
    }
    for (ex, ey) in [(16, 23), (28, 23)] {
        for dy in -2..=2 {
            for dx in -1..=1 {
                put(ex + dx, ey + dy, dark);
            }
        }
    }
    for dx in -1..=1 {
        put(22 + dx, 29, dark);
    }
    Icon::from_rgba(px, N as u32, N as u32).expect("icon")
}
