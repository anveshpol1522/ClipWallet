use rdev::Key;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyAction {
    // Static mode
    StaticCopy(usize),
    StaticPaste(usize),
    StaticCut(usize),
    StaticNavigateNext,
    StaticNavigatePrev,
    StaticDeleteCurrent,

    // Dynamic mode
    DynamicCopy,
    DynamicCut,
    DynamicPaste,           // Cmd+Opt+V in dynamic mode — pastes cursor entry
    DynamicNavigateNext,
    DynamicNavigatePrev,
    DynamicDeleteCurrent,

    // Internal
    CursorReset,
}

pub struct ChordDetector {
    held:         HashSet<Key>,
    /// Prevents Tab from firing repeatedly while held down
    tab_consumed: bool,
}

impl ChordDetector {
    pub fn new() -> Self {
        Self {
            held:         HashSet::new(),
            tab_consumed: false,
        }
    }

    pub fn key_down(&mut self, key: Key) {
        self.held.insert(key);
    }

    pub fn key_up(&mut self, key: Key) {
        if key == Key::Tab {
            self.tab_consumed = false; // reset so next press fires again
        }
        self.held.remove(&key);
    }

    // ── Modifier helpers ──────────────────────────────────────────────

    pub fn cmd(&self) -> bool {
        self.held.contains(&Key::MetaLeft)
            || self.held.contains(&Key::MetaRight)
    }

    pub fn opt(&self) -> bool {
        self.held.contains(&Key::Alt)
            || self.held.contains(&Key::AltGr)
    }

    fn shift(&self) -> bool {
        self.held.contains(&Key::ShiftLeft)
            || self.held.contains(&Key::ShiftRight)
    }

    // ── Called on KeyPress ────────────────────────────────────────────
    // Only Tab fires on press — one action per physical press.
    // C / V / X are handled in evaluate_release.
    pub fn evaluate_press(
        &mut self,
        key: &Key,
        mode_static: bool,
    ) -> Option<HotkeyAction> {
        if !self.cmd() || !self.opt() {
            return None;
        }

        match key {
            Key::Tab => {
                if self.tab_consumed {
                    return None; // ignore held-down repeats
                }
                self.tab_consumed = true;

                if self.held.contains(&Key::Escape) {
                    return Some(if mode_static {
                        HotkeyAction::StaticDeleteCurrent
                    } else {
                        HotkeyAction::DynamicDeleteCurrent
                    });
                }

                if self.shift() {
                    Some(if mode_static {
                        HotkeyAction::StaticNavigatePrev
                    } else {
                        HotkeyAction::DynamicNavigatePrev
                    })
                } else {
                    Some(if mode_static {
                        HotkeyAction::StaticNavigateNext
                    } else {
                        HotkeyAction::DynamicNavigateNext
                    })
                }
            }

            Key::Escape if self.held.contains(&Key::Tab) => {
                Some(if mode_static {
                    HotkeyAction::StaticDeleteCurrent
                } else {
                    HotkeyAction::DynamicDeleteCurrent
                })
            }

            _ => None,
        }
    }

    // ── Called on KeyRelease ──────────────────────────────────────────
    // C / V / X fire here — user must fully press AND release the key
    // while Cmd+Opt is still held.
    pub fn evaluate_release(
        &self,
        key: &Key,
        mode_static: bool,
    ) -> Option<HotkeyAction> {
        if !self.cmd() || !self.opt() {
            return None;
        }

        match key {
            // ── Copy ──────────────────────────────────────────────────
            Key::KeyC => {
                if mode_static {
                    self.digit_held().map(HotkeyAction::StaticCopy)
                } else {
                    Some(HotkeyAction::DynamicCopy)
                }
            }

            // ── Cut ───────────────────────────────────────────────────
            Key::KeyX => {
                if mode_static {
                    self.digit_held().map(HotkeyAction::StaticCut)
                } else {
                    Some(HotkeyAction::DynamicCut)
                }
            }

            // ── Paste ─────────────────────────────────────────────────
            // Static: Cmd+Opt+V+[1-9] pastes from numbered slot
            // Dynamic: Cmd+Opt+V pastes the current cursor entry
            Key::KeyV => {
                if mode_static {
                    self.digit_held().map(HotkeyAction::StaticPaste)
                } else {
                    Some(HotkeyAction::DynamicPaste)
                }
            }

            _ => None,
        }
    }

    fn digit_held(&self) -> Option<usize> {
        [
            (Key::Num1, 1), (Key::Num2, 2), (Key::Num3, 3),
            (Key::Num4, 4), (Key::Num5, 5), (Key::Num6, 6),
            (Key::Num7, 7), (Key::Num8, 8), (Key::Num9, 9),
        ]
        .iter()
        .find(|(k, _)| self.held.contains(k))
        .map(|(_, d)| *d)
    }
}