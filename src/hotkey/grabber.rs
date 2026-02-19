//! macOS CGEventTap — intercepts and suppresses key events.
//! Modifier keys arrive via FlagsChanged, NOT KeyDown/KeyUp.
//! Injected Cmd+C/X events are identified and passed through cleanly.

use crate::hotkey::{ChordDetector, HotkeyAction};
use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation,
    CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField,
};
use std::cell::RefCell;
use std::sync::mpsc::SyncSender;
use tracing::{debug, info};

pub fn start_event_tap(tx: SyncSender<HotkeyAction>, mode_static: bool) {
    let detector     = RefCell::new(ChordDetector::new());
    // One-shot flag: ignore the next KeyC or KeyX that arrives
    // after we inject simulate_copy / simulate_cut
    let ignore_next_c = RefCell::new(false);
    let ignore_next_x = RefCell::new(false);

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ],
        move |_proxy, event_type, event| {
            handle_event(
                &mut detector.borrow_mut(),
                &mut ignore_next_c.borrow_mut(),
                &mut ignore_next_x.borrow_mut(),
                &tx,
                event_type,
                event,
                mode_static,
            )
        },
    )
    .expect("CGEventTap creation failed — ensure Accessibility permission is granted");

    let loop_src = tap
        .mach_port
        .create_runloop_source(0)
        .expect("Failed to create CFRunLoop source");

    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&loop_src, unsafe { kCFRunLoopCommonModes });
    tap.enable();

    info!("CGEventTap active — key interception running");
    CFRunLoop::run_current();
}

fn handle_event(
    detector:      &mut ChordDetector,
    ignore_next_c: &mut bool,
    ignore_next_x: &mut bool,
    tx:            &SyncSender<HotkeyAction>,
    event_type:    CGEventType,
    event:         &CGEvent,
    mode_static:   bool,
) -> Option<CGEvent> {
    let flags = event.get_flags();
    let cmd   = flags.contains(CGEventFlags::CGEventFlagCommand);
    let opt   = flags.contains(CGEventFlags::CGEventFlagAlternate);

    match event_type {

        // ── FlagsChanged ──────────────────────────────────────────────
        CGEventType::FlagsChanged => {
            sync_modifiers(detector, flags);
            debug!("Modifiers → cmd={} opt={}", cmd, opt);
            Some(event.to_owned())
        }

        // ── KeyDown ───────────────────────────────────────────────────
        CGEventType::KeyDown => {
            let is_repeat = event
                .get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT)
                == 1;

            if is_repeat && cmd && opt {
                return None;
            }

            let keycode = event
                .get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
                as u16;

            let key = match keycode_to_rdev(keycode) {
                Some(k) => k,
                None    => return Some(event.to_owned()),
            };

            // ── Injection guard ───────────────────────────────────────
            // If we injected Cmd+C or Cmd+X, the next KeyC / KeyX that
            // arrives is our own injection — let it pass through and
            // clear the flag so normal copies work afterward.
            if key == rdev::Key::KeyC && *ignore_next_c {
                *ignore_next_c = false;
                debug!("Passing injected Cmd+C through");
                return Some(event.to_owned()); // pass to target app
            }
            if key == rdev::Key::KeyX && *ignore_next_x {
                *ignore_next_x = false;
                debug!("Passing injected Cmd+X through");
                return Some(event.to_owned());
            }

            detector.key_down(key.clone());
            debug!("KeyDown {:?}  cmd={} opt={}", key, cmd, opt);

            if cmd && opt {
                // Set ignore flag BEFORE firing DynamicCopy/Cut
                // so when simulate_copy/simulate_cut fires Cmd+C/X
                // we pass it straight through
                if key == rdev::Key::KeyC {
                    *ignore_next_c = true;
                }
                if key == rdev::Key::KeyX {
                    *ignore_next_x = true;
                }

                if let Some(action) = detector.evaluate_press(&key, mode_static) {
                    debug!("Action (press): {:?}", action);
                    let _ = tx.send(action);
                }
                return None;
            }

            Some(event.to_owned())
        }

        // ── KeyUp ─────────────────────────────────────────────────────
        CGEventType::KeyUp => {
            let keycode = event
                .get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
                as u16;

            let key = match keycode_to_rdev(keycode) {
                Some(k) => k,
                None    => return Some(event.to_owned()),
            };

            let was_clipwallet = cmd && opt;

            if was_clipwallet {
                if let Some(action) = detector.evaluate_release(&key, mode_static) {
                    debug!("Action (release): {:?}", action);
                    let _ = tx.send(action);
                }
            }

            detector.key_up(key);

            if was_clipwallet {
                return None;
            }

            Some(event.to_owned())
        }

        _ => Some(event.to_owned()),
    }
}

fn sync_modifiers(detector: &mut ChordDetector, flags: CGEventFlags) {
    use rdev::Key;

    if flags.contains(CGEventFlags::CGEventFlagCommand) {
        detector.key_down(Key::MetaLeft);
    } else {
        detector.key_up(Key::MetaLeft);
        detector.key_up(Key::MetaRight);
    }

    if flags.contains(CGEventFlags::CGEventFlagAlternate) {
        detector.key_down(Key::Alt);
    } else {
        detector.key_up(Key::Alt);
        detector.key_up(Key::AltGr);
    }

    if flags.contains(CGEventFlags::CGEventFlagShift) {
        detector.key_down(Key::ShiftLeft);
    } else {
        detector.key_up(Key::ShiftLeft);
        detector.key_up(Key::ShiftRight);
    }
}

fn keycode_to_rdev(code: u16) -> Option<rdev::Key> {
    use rdev::Key::*;
    Some(match code {
        0  => KeyA,  1  => KeyS,  2  => KeyD,  3  => KeyF,
        4  => KeyH,  5  => KeyG,  6  => KeyZ,  7  => KeyX,
        8  => KeyC,  9  => KeyV,  11 => KeyB,  12 => KeyQ,
        13 => KeyW,  14 => KeyE,  15 => KeyR,  16 => KeyY,
        17 => KeyT,  31 => KeyO,  32 => KeyU,  34 => KeyI,
        35 => KeyP,  37 => KeyL,  38 => KeyJ,  40 => KeyK,
        45 => KeyN,  46 => KeyM,

        18 => Num1,  19 => Num2,  20 => Num3,
        21 => Num4,  23 => Num5,  22 => Num6,
        26 => Num7,  28 => Num8,  25 => Num9,
        29 => Num0,

        48  => Tab,
        53  => Escape,
        36  => Return,
        51  => Backspace,
        49  => Space,

        123 => LeftArrow,
        124 => RightArrow,
        125 => DownArrow,
        126 => UpArrow,

        _ => return None,
    })
}