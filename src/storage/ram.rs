use crate::clipboard::types::{ClipEntry, EntryId};
use std::collections::VecDeque;
use std::time::Instant;

pub const MAX_DYNAMIC:  usize = 50;
pub const STATIC_SLOTS: usize = 9;

/// Cursor timeout in seconds — resets to most recent after this idle period
pub const CURSOR_TIMEOUT_SECS: u64 = 10;

static mut ID_COUNTER: EntryId = 0;

pub fn next_id() -> EntryId {
    unsafe {
        ID_COUNTER += 1;
        ID_COUNTER
    }
}

pub struct RamStore {
    pub static_slots:    [Option<ClipEntry>; STATIC_SLOTS],
    pub dynamic_ring:    VecDeque<ClipEntry>,
    pub dynamic_cursor:  usize,

    /// Tracks when the user last navigated — used by timeout task
    pub last_nav_time:   Instant,

    pub dirty: bool,
}

impl RamStore {
    pub fn new() -> Self {
        Self {
            static_slots:  std::array::from_fn(|_| None),
            dynamic_ring:  VecDeque::with_capacity(MAX_DYNAMIC),
            dynamic_cursor: 0,
            last_nav_time: Instant::now(),
            dirty: false,
        }
    }

    // ── Static ────────────────────────────────────────────────────────

    pub fn set_static(&mut self, slot: usize, entry: ClipEntry) {
        assert!(slot >= 1 && slot <= 9);
        self.static_slots[slot - 1] = Some(entry);
        self.dirty = true;
    }

    pub fn get_static(&self, slot: usize) -> Option<&ClipEntry> {
        assert!(slot >= 1 && slot <= 9);
        self.static_slots[slot - 1].as_ref()
    }

    pub fn clear_static(&mut self, slot: usize) {
        assert!(slot >= 1 && slot <= 9);
        self.static_slots[slot - 1] = None;
        self.dirty = true;
    }

    // ── Dynamic ───────────────────────────────────────────────────────

    /// Push to front. Returns the evicted entry label if ring was full.
    pub fn push_dynamic(&mut self, entry: ClipEntry) -> Option<String> {
        let mut evicted = None;
        if self.dynamic_ring.len() >= MAX_DYNAMIC {
            if let Some(old) = self.dynamic_ring.pop_back() {
                evicted = Some(format!(
                    "id={} ({})",
                    old.id,
                    old.data.type_label()
                ));
            }
        }
        self.dynamic_ring.push_front(entry);
        self.dynamic_cursor = 0;
        self.dirty = true;
        evicted
    }

    pub fn current_dynamic(&self) -> Option<&ClipEntry> {
        self.dynamic_ring.get(self.dynamic_cursor)
    }

    /// Move cursor forward (→ older). Wraps around.
    pub fn cursor_next(&mut self) {
        if self.dynamic_ring.is_empty() { return; }
        self.dynamic_cursor = (self.dynamic_cursor + 1) % self.dynamic_ring.len();
        self.last_nav_time = Instant::now();
    }

    /// Move cursor backward (→ newer). Wraps around.
    pub fn cursor_prev(&mut self) {
        if self.dynamic_ring.is_empty() { return; }
        let len = self.dynamic_ring.len();
        self.dynamic_cursor = if self.dynamic_cursor == 0 {
            len - 1
        } else {
            self.dynamic_cursor - 1
        };
        self.last_nav_time = Instant::now();
    }

    pub fn delete_at_cursor(&mut self) {
        if self.dynamic_cursor < self.dynamic_ring.len() {
            self.dynamic_ring.remove(self.dynamic_cursor);
            if self.dynamic_cursor >= self.dynamic_ring.len() && self.dynamic_cursor > 0 {
                self.dynamic_cursor -= 1;
            }
            self.dirty = true;
        }
    }

    /// Reset cursor to most recent (index 0)
    pub fn reset_cursor(&mut self) {
        self.dynamic_cursor = 0;
    }

    /// Returns true if cursor has been idle longer than CURSOR_TIMEOUT_SECS
    pub fn cursor_timed_out(&self) -> bool {
        self.last_nav_time.elapsed().as_secs() >= CURSOR_TIMEOUT_SECS
    }

    /// How many entries are in the ring
    pub fn ring_len(&self) -> usize {
        self.dynamic_ring.len()
    }

    pub fn clear_dirty(&mut self) { self.dirty = false; }
    pub fn is_dirty(&self)        -> bool { self.dirty }
}