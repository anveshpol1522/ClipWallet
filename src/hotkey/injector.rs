use rdev::{simulate, EventType, Key};
use std::thread::sleep;
use std::time::Duration;
use tracing::error;

/// Fire a keydown + keyup pair
fn press(key: Key) {
    let down = simulate(&EventType::KeyPress(key.clone()));
    sleep(Duration::from_millis(20));
    let up = simulate(&EventType::KeyRelease(key));
    if down.is_err() || up.is_err() {
        error!("Key injection failed for {:?}", key);
    }
}

/// Inject Cmd+C to copy selected text into system clipboard.
/// Does NOT hold Opt so our CGEventTap passes it through cleanly.
pub fn simulate_copy() {
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyC);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
    // Wait for the target app to process the copy before we read clipboard
    sleep(Duration::from_millis(120));
}

/// Inject Cmd+X to cut selected text into system clipboard.
pub fn simulate_cut() {
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyX);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
    sleep(Duration::from_millis(120));
}

/// Inject Cmd+V to paste.
/// Call this only after confirming Opt key is released,
/// otherwise our CGEventTap will suppress it.
pub fn simulate_paste() {
    sleep(Duration::from_millis(50));
    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    sleep(Duration::from_millis(20));
    press(Key::KeyV);
    sleep(Duration::from_millis(20));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
}

/// Inject Cmd+V after a delay to ensure all modifier keys are released.
/// Used by dynamic paste so the injected Cmd+V is not suppressed by our tap.
pub fn simulate_paste_delayed() {
    std::thread::spawn(|| {
        // 300ms gives enough time for user to release Cmd+Opt after the hotkey
        sleep(Duration::from_millis(300));
        simulate_paste();
    });
}