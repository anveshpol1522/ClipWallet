//! StaticSlotStore — fixed addressed clipboard slots for static mode.
//!
//! 9 independent slots, addressed directly by digit 1-9.
//! Slot N is always at index N-1. Slots are never reordered.
//! Unwritten slots are None and displayed as NULL in logs.
//!
//! Tab/Shift+Tab move a read cursor which syncs the slot at that
//! position to the system clipboard (making Cmd+V paste it).
//! The cursor never rearranges data.

use crate::clipboard::types::ClipEntry;

pub const NUM_SLOTS: usize = 9;

pub struct StaticSlotStore {
    /// Fixed array. slots[0] = slot 1, slots[8] = slot 9.
    /// None = empty/NULL. Never grows or reorders.
    pub slots: [Option<ClipEntry>; NUM_SLOTS],

    /// Current Tab-cursor position (0-based).
    /// Only used for Tab navigation — never affects slot addresses.
    pub cursor: usize,
}

impl StaticSlotStore {
    pub fn new() -> Self {
        Self {
            slots:  std::array::from_fn(|_| None),
            cursor: 0,
        }
    }

    // ── Direct slot access ────────────────────────────────────────────

    /// Write entry to slot N (1-based). Overwrites whatever was there.
    pub fn write(&mut self, slot: usize, entry: ClipEntry) {
        assert!(slot >= 1 && slot <= NUM_SLOTS, "Slot out of range: {}", slot);
        self.slots[slot - 1] = Some(entry);
    }

    /// Read from slot N (1-based). Returns None if empty.
    pub fn read(&self, slot: usize) -> Option<&ClipEntry> {
        assert!(slot >= 1 && slot <= NUM_SLOTS, "Slot out of range: {}", slot);
        self.slots[slot - 1].as_ref()
    }

    /// Clear slot N (1-based) → NULL.
    pub fn clear(&mut self, slot: usize) {
        assert!(slot >= 1 && slot <= NUM_SLOTS, "Slot out of range: {}", slot);
        self.slots[slot - 1] = None;
    }

    // ── Tab cursor navigation ─────────────────────────────────────────
    // Cursor skips NULL slots. Returns the slot number (1-based)
    // it landed on, or None if all slots are empty.

    pub fn cursor_next(&mut self) -> Option<usize> {
        self.move_cursor(1)
    }

    pub fn cursor_prev(&mut self) -> Option<usize> {
        self.move_cursor(-1)
    }

    fn move_cursor(&mut self, direction: i32) -> Option<usize> {
        let start = self.cursor;
        let mut cur = self.cursor;

        for _ in 0..NUM_SLOTS {
            let next = if direction > 0 {
                (cur + 1) % NUM_SLOTS
            } else {
                if cur == 0 { NUM_SLOTS - 1 } else { cur - 1 }
            };
            cur = next;
            if self.slots[cur].is_some() {
                self.cursor = cur;
                return Some(cur + 1); // return 1-based slot number
            }
            if cur == start { break; }
        }
        None // all slots empty
    }

    /// Entry at the current cursor position (1-based slot number).
    pub fn cursor_slot(&self) -> usize {
        self.cursor + 1
    }

    pub fn cursor_entry(&self) -> Option<&ClipEntry> {
        self.slots[self.cursor].as_ref()
    }

    pub fn delete_at_cursor(&mut self) {
        self.slots[self.cursor] = None;
    }

    // ── Status ────────────────────────────────────────────────────────

    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }
}
