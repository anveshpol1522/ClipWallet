use crate::clipboard::mime::{detect_bytes, normalise_image, DetectedType};
use crate::clipboard::pasteboard;
use crate::clipboard::types::{ClipData, ClipEntry};
use crate::hotkey::{
    simulate_copy, simulate_cut, simulate_paste, simulate_paste_delayed, HotkeyAction,
};
use crate::storage::encrypted::{load_from_vault, save_to_vault};
use crate::storage::ram::{next_id, RamStore};
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

const PASTE_SETTLE_MS: u64 = 60;

pub struct Engine {
    ram:           Arc<RwLock<RamStore>>,
    clipboard:     Clipboard,
    static_cursor: usize,
}

impl Engine {
    pub fn new(ram: Arc<RwLock<RamStore>>) -> anyhow::Result<Self> {
        Ok(Self {
            ram,
            clipboard:     Clipboard::new()?,
            static_cursor: 0,
        })
    }

    pub fn handle(&mut self, action: HotkeyAction) {
        info!("[ACTION] {:?}", action);
        match action {
            HotkeyAction::StaticCopy(s)       => self.static_copy(s),
            HotkeyAction::StaticCut(s)        => self.static_cut(s),
            HotkeyAction::StaticPaste(s)      => self.static_paste(s),
            HotkeyAction::StaticNavigateNext  => self.static_nav(1),
            HotkeyAction::StaticNavigatePrev  => self.static_nav(-1),
            HotkeyAction::StaticDeleteCurrent => self.static_delete(),
            HotkeyAction::DynamicCopy         => self.dynamic_copy(),
            HotkeyAction::DynamicCut          => self.dynamic_cut(),
            HotkeyAction::DynamicPaste        => self.dynamic_paste(),
            HotkeyAction::DynamicNavigateNext => self.dynamic_nav(1),
            HotkeyAction::DynamicNavigatePrev => self.dynamic_nav(-1),
            HotkeyAction::DynamicDeleteCurrent=> self.dynamic_delete(),
            HotkeyAction::CursorReset         => self.cursor_reset(),
        }
    }

    // ── Clipboard read ────────────────────────────────────────────────

    fn read_clipboard(&mut self) -> Option<ClipData> {
        if let Some(paths) = pasteboard::read_file_paths() {
            info!("Read {} file path(s) from clipboard", paths.len());
            return Some(ClipData::FilePath(paths));
        }
        if let Some(rtf) = pasteboard::read_rtf() {
            info!("Read RTF ({} bytes)", rtf.len());
            return Some(ClipData::RichText(rtf));
        }
        if let Ok(img) = self.clipboard.get_image() {
            let raw: Vec<u8> = img.bytes.into_owned();
            match normalise_image(&raw) {
                Ok(png) => {
                    info!("Read Image → PNG ({} bytes)", png.len());
                    return Some(ClipData::Image {
                        bytes: png, width: img.width, height: img.height,
                    });
                }
                Err(e) => warn!("Image normalise failed: {}", e),
            }
            return Some(ClipData::Binary(raw));
        }
        if let Ok(text) = self.clipboard.get_text() {
            if !text.is_empty() {
                info!("Read PlainText ({} chars)", text.len());
                return Some(ClipData::PlainText(text));
            }
        }
        warn!("Clipboard empty or unreadable");
        None
    }

    // ── Sync to system clipboard (no paste injection) ─────────────────

    fn sync_to_system_clipboard(&mut self, data: &ClipData) -> bool {
        match data {
            ClipData::PlainText(text) => {
                match self.clipboard.set_text(text.clone()) {
                    Ok(_)  => { debug!("Synced PlainText ({} chars)", text.len()); true }
                    Err(e) => { warn!("Clipboard sync failed: {}", e); false }
                }
            }
            ClipData::RichText(bytes) => {
                let ok = pasteboard::write_rtf(bytes);
                if !ok { warn!("RTF sync failed"); }
                ok
            }
            ClipData::Image { bytes, width, height } => {
                let img = ImageData {
                    bytes: Cow::Borrowed(bytes), width: *width, height: *height,
                };
                match self.clipboard.set_image(img) {
                    Ok(_)  => true,
                    Err(e) => { warn!("Image sync failed: {}", e); false }
                }
            }
            ClipData::FilePath(paths) => {
                let ok = pasteboard::write_file_paths(paths);
                if !ok { warn!("FilePath sync failed"); }
                ok
            }
            ClipData::Binary(bytes) => {
                match detect_bytes(bytes) {
                    DetectedType::PlainText => {
                        let t = String::from_utf8_lossy(bytes).to_string();
                        self.clipboard.set_text(t).is_ok()
                    }
                    _ => { warn!("Binary type cannot be synced"); false }
                }
            }
        }
    }

    // ── Static write + inject Cmd+V (used in static mode only) ────────

    fn write_and_paste(&mut self, entry: &ClipEntry) {
        if self.sync_to_system_clipboard(&entry.data) {
            std::thread::sleep(std::time::Duration::from_millis(PASTE_SETTLE_MS));
            simulate_paste();
        }
    }

    // ── Debug helpers ─────────────────────────────────────────────────

    fn entry_preview(&self, entry: &ClipEntry) -> String {
        match &entry.data {
            ClipData::PlainText(t) => {
                let s: String = t.chars().take(40).collect();
                let s = s.replace('\n', "↵");
                if t.len() > 40 { format!("\"{}…\"", s) }
                else            { format!("\"{}\"", s) }
            }
            ClipData::Image { width, height, .. } =>
                format!("[Image {}x{}]", width, height),
            ClipData::FilePath(p) =>
                format!("[{} file(s)]", p.len()),
            ClipData::RichText(b)  => format!("[RTF {} bytes]", b.len()),
            ClipData::Binary(b)    => format!("[Binary {} bytes]", b.len()),
        }
    }

    fn log_ring_state(&self) {
        let ram = self.ram.read().unwrap();
        let len = ram.ring_len();
        let cur = ram.dynamic_cursor;
        debug!("┌─ Ring State ({} entries) ─────────────────────────", len);
        for (i, entry) in ram.dynamic_ring.iter().enumerate() {
            let preview = self.entry_preview(entry);
            let marker  = if i == cur { "►" } else { " " };
            debug!("│ {} [{}] id={} type={} | {}",
                marker, i, entry.id, entry.data.type_label(), preview);
        }
        if len == 0 { debug!("│   (empty)"); }
        debug!("└─ Cursor [{}] — live in Cmd+V ──────────────────────", cur);
    }

    fn log_static_state(&self) {
        let ram = self.ram.read().unwrap();
        debug!("┌─ Static Slots ──────────────────────────────────────");
        for i in 0..9 {
            let marker = if i == self.static_cursor { "►" } else { " " };
            match &ram.static_slots[i] {
                Some(e) => debug!("│ {} slot {} id={} type={} | {}",
                    marker, i+1, e.id, e.data.type_label(), self.entry_preview(e)),
                None    => debug!("│ {} slot {} — empty", marker, i+1),
            }
        }
        debug!("└─────────────────────────────────────────────────────");
    }

    // ── Dynamic operations ────────────────────────────────────────────

    /// Cmd+Opt+C:
    /// Injects clean Cmd+C (no Opt) → app copies selection to clipboard
    /// → we read it → push to ring[0] → sync back to clipboard.
    /// Plain Cmd+V immediately pastes ring[0].
    fn dynamic_copy(&mut self) {
        info!("[Dynamic][COPY] Injecting Cmd+C to capture selection...");
        simulate_copy();

        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Dynamic][COPY] Captured id={} type={} size={}B → ring[0]",
                entry.id, entry.data.type_label(), entry.data.size_bytes());

            let evicted = self.ram.write().unwrap().push_dynamic(entry.clone());
            if let Some(old) = evicted {
                info!("[Dynamic][EVICT] Dropped oldest: {}", old);
            }
            self.sync_to_system_clipboard(&entry.data);
            self.log_ring_state();
        }
    }

    /// Cmd+Opt+X:
    /// Injects clean Cmd+X (no Opt) → app cuts selection to clipboard
    /// → we read it → push to ring[0] → sync back to clipboard.
    fn dynamic_cut(&mut self) {
        info!("[Dynamic][CUT] Injecting Cmd+X to cut selection...");
        simulate_cut();

        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Dynamic][CUT] Captured id={} type={} size={}B → ring[0]",
                entry.id, entry.data.type_label(), entry.data.size_bytes());

            self.ram.write().unwrap().push_dynamic(entry.clone());
            self.sync_to_system_clipboard(&entry.data);
            self.log_ring_state();
        }
    }

    /// Cmd+Opt+V:
    /// Re-syncs cursor entry to clipboard then schedules Cmd+V injection
    /// after 300ms — by then Opt is released so our tap won't suppress it.
    fn dynamic_paste(&mut self) {
        let entry = self.ram.read().unwrap().current_dynamic().cloned();
        match entry {
            Some(e) => {
                let cursor = self.ram.read().unwrap().dynamic_cursor;
                info!("[Dynamic][PASTE] ring[{}] id={} type={} — syncing clipboard",
                    cursor, e.id, e.data.type_label());
                self.sync_to_system_clipboard(&e.data);
                // Delayed injection — 300ms ensures Cmd+Opt are released
                // before Cmd+V fires so our CGEventTap won't suppress it
                simulate_paste_delayed();
                info!("[Dynamic][PASTE] Cmd+V scheduled in 300ms");
            }
            None => warn!("[Dynamic][PASTE] Ring is empty"),
        }
    }

    /// Cmd+Opt+Tab / Shift+Tab:
    /// Move cursor, sync new cursor entry to clipboard.
    /// Plain Cmd+V pastes it immediately after.
    fn dynamic_nav(&mut self, direction: i32) {
        {
            let mut ram = self.ram.write().unwrap();
            if direction > 0 { ram.cursor_next(); }
            else             { ram.cursor_prev(); }
        }
        let ram    = self.ram.read().unwrap();
        let cursor = ram.dynamic_cursor;
        let total  = ram.ring_len();

        if let Some(entry) = ram.current_dynamic().cloned() {
            info!("[Dynamic][NAV] [{}/{}] id={} type={} — synced to Cmd+V",
                cursor + 1, total, entry.id, entry.data.type_label());
            drop(ram);
            self.sync_to_system_clipboard(&entry.data);
            self.log_ring_state();
        }
    }

    /// Cmd+Opt+Tab+Esc: delete cursor entry, sync next to clipboard.
    fn dynamic_delete(&mut self) {
        let cursor = self.ram.read().unwrap().dynamic_cursor;
        let id     = self.ram.read().unwrap().current_dynamic().map(|e| e.id);
        self.ram.write().unwrap().delete_at_cursor();
        info!("[Dynamic][DELETE] Removed ring[{}] id={:?}", cursor, id);
        let entry = self.ram.read().unwrap().current_dynamic().cloned();
        if let Some(e) = entry {
            info!("[Dynamic][DELETE] New cursor: id={}", e.id);
            self.sync_to_system_clipboard(&e.data);
        }
        self.log_ring_state();
    }

    /// Internal: reset cursor to ring[0] after idle timeout.
    fn cursor_reset(&mut self) {
        let was_at = self.ram.read().unwrap().dynamic_cursor;
        if was_at != 0 {
            self.ram.write().unwrap().reset_cursor();
            info!("[Dynamic][TIMEOUT] Cursor reset from [{}] → [0]", was_at + 1);
            let entry = self.ram.read().unwrap().current_dynamic().cloned();
            if let Some(e) = entry {
                self.sync_to_system_clipboard(&e.data);
            }
            self.log_ring_state();
        }
    }

    // ── Static operations ─────────────────────────────────────────────

    fn static_copy(&mut self, slot: usize) {
        simulate_copy();
        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Static][COPY] slot={} id={} type={}",
                slot, entry.id, entry.data.type_label());
            self.ram.write().unwrap().set_static(slot, entry);
            self.log_static_state();
        }
    }

    fn static_cut(&mut self, slot: usize) {
        simulate_cut();
        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Static][CUT] slot={} id={}", slot, entry.id);
            self.ram.write().unwrap().set_static(slot, entry);
            self.log_static_state();
        }
    }

    fn static_paste(&mut self, slot: usize) {
        let entry = self.ram.read().unwrap().get_static(slot).cloned();
        match entry {
            Some(e) => {
                info!("[Static][PASTE] slot={} id={} type={}",
                    slot, e.id, e.data.type_label());
                self.write_and_paste(&e);
            }
            None => warn!("[Static][PASTE] slot={} is empty", slot),
        }
    }

    fn static_nav(&mut self, direction: i32) {
        let ram   = self.ram.read().unwrap();
        let start = self.static_cursor;
        let mut cur = self.static_cursor;
        loop {
            let next = if direction > 0 { (cur + 1) % 9 }
                       else { if cur == 0 { 8 } else { cur - 1 } };
            if ram.static_slots[next].is_some() { self.static_cursor = next; break; }
            cur = next;
            if cur == start { break; }
        }
        if let Some(entry) = ram.static_slots[self.static_cursor].clone() {
            drop(ram);
            info!("[Static][NAV] slot={} id={}", self.static_cursor + 1, entry.id);
            self.sync_to_system_clipboard(&entry.data);
            self.log_static_state();
        }
    }

    fn static_delete(&mut self) {
        let slot = self.static_cursor + 1;
        self.ram.write().unwrap().clear_static(slot);
        info!("[Static][DELETE] slot={}", slot);
        self.log_static_state();
    }

    // ── Vault ─────────────────────────────────────────────────────────

    pub fn encrypt_slot(&mut self, slot: usize) {
        let entry = self.ram.read().unwrap().get_static(slot).cloned();
        match entry {
            Some(mut e) => {
                e.encrypted = true;
                match save_to_vault(&e) {
                    Ok(path) => {
                        self.ram.write().unwrap().clear_static(slot);
                        info!("[Vault] Encrypted slot {} → {:?}", slot, path);
                    }
                    Err(e) => warn!("[Vault] Encrypt failed: {}", e),
                }
            }
            None => warn!("[Vault] Slot {} empty", slot),
        }
    }

    pub fn decrypt_slot(&mut self, id: u64, slot: usize) {
        match load_from_vault(id) {
            Ok(mut entry) => {
                entry.encrypted = false;
                info!("[Vault] Decrypted id={} → slot {}", id, slot);
                self.ram.write().unwrap().set_static(slot, entry);
            }
            Err(e) => warn!("[Vault] Decrypt failed: {}", e),
        }
    }
}