use rdev::{simulate, EventType, Key};
use std::thread::sleep;
use std::time::Duration;
use tracing::error;

fn press(key: Key) {
    let _ = simulate(&EventType::KeyPress(key.clone()));
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(key));
}

/// Inject Cmd+C cleanly.
/// We explicitly release Opt FIRST so the tap sees cmd=true opt=false
/// and does not suppress or re-trigger DynamicCopy.
pub fn simulate_copy() {
    // Release Opt first — ensures our tap won't catch this as Cmd+Opt+C
    let _ = simulate(&EventType::KeyRelease(Key::Alt));
    let _ = simulate(&EventType::KeyRelease(Key::AltGr));
    sleep(Duration::from_millis(30));

    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyC);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));

    // Wait for the target app to process the copy
    sleep(Duration::from_millis(150));
}

/// Inject Cmd+X cleanly.
/// Same as simulate_copy — releases Opt first.
pub fn simulate_cut() {
    let _ = simulate(&EventType::KeyRelease(Key::Alt));
    let _ = simulate(&EventType::KeyRelease(Key::AltGr));
    sleep(Duration::from_millis(30));

    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyX);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));

    sleep(Duration::from_millis(150));
}

/// Inject Cmd+V for static mode paste.
pub fn simulate_paste() {
    sleep(Duration::from_millis(50));
    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyV);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
}

/// Inject Cmd+V after a delay so Cmd+Opt are fully released first.
/// Used by dynamic paste — prevents our tap from suppressing the injection.
pub fn simulate_paste_delayed() {
    std::thread::spawn(|| {
        sleep(Duration::from_millis(350));
        simulate_paste();
    });
}