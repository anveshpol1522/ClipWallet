use crate::clipboard::mime::{detect_bytes, normalise_image, DetectedType};
use crate::clipboard::pasteboard;
use crate::clipboard::types::{ClipData, ClipEntry};
use crate::hotkey::{
    simulate_copy, simulate_cut, simulate_paste, simulate_paste_delayed, HotkeyAction,
};
use crate::static_store::StaticSlotStore;
use crate::storage::encrypted::{load_from_vault, save_to_vault};
use crate::storage::ram::{next_id, RamStore};
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

const PASTE_SETTLE_MS: u64 = 60;

pub struct Engine {
    ram:          Arc<RwLock<RamStore>>,
    clipboard:    Clipboard,
    static_store: StaticSlotStore,  // dedicated fixed-array store, not shared
}

impl Engine {
    pub fn new(ram: Arc<RwLock<RamStore>>) -> anyhow::Result<Self> {
        Ok(Self {
            ram,
            clipboard:    Clipboard::new()?,
            static_store: StaticSlotStore::new(),
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
            info!("Read {} file path(s)", paths.len());
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

    // ── Sync to system clipboard (no injection) ───────────────────────

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
                    _ => { warn!("Binary cannot be synced"); false }
                }
            }
        }
    }

    fn write_and_paste(&mut self, data: &ClipData) {
        if self.sync_to_system_clipboard(data) {
            std::thread::sleep(std::time::Duration::from_millis(PASTE_SETTLE_MS));
            simulate_paste();
        }
    }

    // ── Debug: print full static slot table ───────────────────────────

    fn log_static_state(&self) {
        debug!("┌─ Static Slots ({}/{} occupied) ────────────────────────",
            self.static_store.occupied_count(), 9);
        for i in 0..9 {
            let slot   = i + 1;
            let marker = if i == self.static_store.cursor { "►" } else { " " };
            match &self.static_store.slots[i] {
                Some(e) => {
                    let preview = self.entry_preview(e);
                    debug!("│ {} slot {} → id={} type={} | {}",
                        marker, slot, e.id, e.data.type_label(), preview);
                }
                None => debug!("│ {} slot {} → NULL", marker, slot),
            }
        }
        debug!("└─────────────────────────────────────────────────────────");
    }

    fn log_ring_state(&self) {
        let ram = self.ram.read().unwrap();
        let len = ram.ring_len();
        let cur = ram.dynamic_cursor;
        debug!("┌─ Ring State ({} entries) ─────────────────────────", len);
        for (i, entry) in ram.dynamic_ring.iter().enumerate() {
            let marker = if i == cur { "►" } else { " " };
            debug!("│ {} [{}] id={} type={} | {}",
                marker, i, entry.id, entry.data.type_label(),
                self.entry_preview(entry));
        }
        if len == 0 { debug!("│   (empty)"); }
        debug!("└─ Cursor [{}] — live in Cmd+V ──────────────────────", cur);
    }

    fn entry_preview(&self, entry: &ClipEntry) -> String {
        match &entry.data {
            ClipData::PlainText(t) => {
                let s: String = t.chars().take(40).collect();
                let s = s.replace('\n', "↵");
                if t.len() > 40 { format!("\"{}…\"", s) }
                else            { format!("\"{}\"", s) }
            }
            ClipData::Image { width, height, .. } => format!("[Image {}x{}]", width, height),
            ClipData::FilePath(p)  => format!("[{} file(s)]", p.len()),
            ClipData::RichText(b)  => format!("[RTF {} bytes]", b.len()),
            ClipData::Binary(b)    => format!("[Binary {} bytes]", b.len()),
        }
    }

    // ════════════════════════════════════════════════════════════════
    // STATIC MODE — Fixed addressed slots
    //
    // Slot 1-9 are permanent array positions.
    // Writing to slot N always goes to index N-1, nowhere else.
    // Tab cursor only moves the read pointer — never reorders data.
    // ════════════════════════════════════════════════════════════════

    /// Cmd+Opt+C+N:
    /// Inject Cmd+C → read clipboard → write to slot N → log full table.
    fn static_copy(&mut self, slot: usize) {
        info!("[Static][COPY] slot={} — injecting Cmd+C", slot);
        simulate_copy();

        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Static][COPY] slot={} ← id={} type={} size={}B",
                slot, entry.id, entry.data.type_label(), entry.data.size_bytes());
            self.static_store.write(slot, entry);
            self.log_static_state();
        }
    }

    /// Cmd+Opt+X+N:
    /// Inject Cmd+X → read clipboard → write to slot N.
    fn static_cut(&mut self, slot: usize) {
        info!("[Static][CUT] slot={} — injecting Cmd+X", slot);
        simulate_cut();

        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Static][CUT] slot={} ← id={} type={}",
                slot, entry.id, entry.data.type_label());
            self.static_store.write(slot, entry);
            self.log_static_state();
        }
    }

    /// Cmd+Opt+V+N:
    /// Sync slot N to clipboard → schedule Cmd+V injection after 300ms.
    /// The 300ms delay ensures Opt is released before injection
    /// so our CGEventTap doesn't suppress the injected Cmd+V.
    fn static_paste(&mut self, slot: usize) {
        let data = self.static_store.read(slot).map(|e| e.data.clone());
        match data {
            Some(d) => {
                info!("[Static][PASTE] slot={} — syncing + scheduling Cmd+V", slot);
                self.sync_to_system_clipboard(&d);
                simulate_paste_delayed();
            }
            None => warn!("[Static][PASTE] slot={} is NULL — nothing to paste", slot),
        }
    }

    /// Cmd+Opt+Tab / Shift+Tab:
    /// Move cursor to next/prev occupied slot.
    /// Sync that slot's content to system clipboard → Cmd+V pastes it.
    fn static_nav(&mut self, direction: i32) {
        let result = if direction > 0 {
            self.static_store.cursor_next()
        } else {
            self.static_store.cursor_prev()
        };

        match result {
            Some(slot) => {
                let data = self.static_store.cursor_entry()
                    .map(|e| (e.id, e.data.type_label(), e.data.clone()));

                if let Some((id, type_label, d)) = data {
                    info!("[Static][NAV] Cursor → slot {} id={} type={} — live in Cmd+V",
                        slot, id, type_label);
                    self.sync_to_system_clipboard(&d);
                    self.log_static_state();
                }
            }
            None => warn!("[Static][NAV] All slots are NULL — nothing to navigate"),
        }
    }

    /// Cmd+Opt+Tab+Esc:
    /// Set slot at current cursor to NULL.
    fn static_delete(&mut self) {
        let slot = self.static_store.cursor_slot();
        let was_some = self.static_store.cursor_entry().is_some();
        if was_some {
            self.static_store.delete_at_cursor();
            info!("[Static][DELETE] slot={} → NULL", slot);
        } else {
            warn!("[Static][DELETE] slot={} already NULL", slot);
        }
        self.log_static_state();
    }

    // ════════════════════════════════════════════════════════════════
    // DYNAMIC MODE — Recency ring
    // ════════════════════════════════════════════════════════════════

    fn dynamic_copy(&mut self) {
        info!("[Dynamic][COPY] Injecting Cmd+C...");
        simulate_copy();
        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Dynamic][COPY] id={} type={} size={}B → ring[0]",
                entry.id, entry.data.type_label(), entry.data.size_bytes());
            let evicted = self.ram.write().unwrap().push_dynamic(entry.clone());
            if let Some(old) = evicted { info!("[Dynamic][EVICT] Dropped: {}", old); }
            self.sync_to_system_clipboard(&entry.data);
            self.log_ring_state();
        }
    }

    fn dynamic_cut(&mut self) {
        info!("[Dynamic][CUT] Injecting Cmd+X...");
        simulate_cut();
        if let Some(data) = self.read_clipboard() {
            let entry = ClipEntry::new(next_id(), data);
            info!("[Dynamic][CUT] id={} type={} → ring[0]",
                entry.id, entry.data.type_label());
            self.ram.write().unwrap().push_dynamic(entry.clone());
            self.sync_to_system_clipboard(&entry.data);
            self.log_ring_state();
        }
    }

    fn dynamic_paste(&mut self) {
        let data = {
            let ram = self.ram.read().unwrap();
            ram.current_dynamic().map(|e| (e.id, e.data.clone()))
        };
        match data {
            Some((id, d)) => {
                let cursor = self.ram.read().unwrap().dynamic_cursor;
                info!("[Dynamic][PASTE] ring[{}] id={} — syncing + scheduling", cursor, id);
                self.sync_to_system_clipboard(&d);
                simulate_paste_delayed();
            }
            None => warn!("[Dynamic][PASTE] Ring is empty"),
        }
    }

    fn dynamic_nav(&mut self, direction: i32) {
        {
            let mut ram = self.ram.write().unwrap();
            if direction > 0 { ram.cursor_next(); } else { ram.cursor_prev(); }
        }
        let data = {
            let ram = self.ram.read().unwrap();
            ram.current_dynamic().map(|e| (e.id, e.data.type_label(), ram.dynamic_cursor, ram.ring_len(), e.data.clone()))
        };
        if let Some((id, type_label, cursor, total, d)) = data {
            info!("[Dynamic][NAV] [{}/{}] id={} type={} — synced to Cmd+V",
                cursor + 1, total, id, type_label);
            self.sync_to_system_clipboard(&d);
            self.log_ring_state();
        }
    }

    fn dynamic_delete(&mut self) {
        let cursor = self.ram.read().unwrap().dynamic_cursor;
        let id     = self.ram.read().unwrap().current_dynamic().map(|e| e.id);
        self.ram.write().unwrap().delete_at_cursor();
        info!("[Dynamic][DELETE] Removed ring[{}] id={:?}", cursor, id);
        let next = self.ram.read().unwrap().current_dynamic().map(|e| e.data.clone());
        if let Some(d) = next { self.sync_to_system_clipboard(&d); }
        self.log_ring_state();
    }

    fn cursor_reset(&mut self) {
        let was_at = self.ram.read().unwrap().dynamic_cursor;
        if was_at != 0 {
            self.ram.write().unwrap().reset_cursor();
            info!("[Dynamic][TIMEOUT] Cursor reset [{}]→[0]", was_at + 1);
            let d = self.ram.read().unwrap().current_dynamic().map(|e| e.data.clone());
            if let Some(data) = d { self.sync_to_system_clipboard(&data); }
            self.log_ring_state();
        }
    }

    // ── Vault ─────────────────────────────────────────────────────────

    pub fn encrypt_slot(&mut self, slot: usize) {
        let entry = self.static_store.read(slot).cloned();
        match entry {
            Some(mut e) => {
                e.encrypted = true;
                match save_to_vault(&e) {
                    Ok(path) => {
                        self.static_store.clear(slot);
                        info!("[Vault] Encrypted slot {} → {:?}", slot, path);
                    }
                    Err(e) => warn!("[Vault] Encrypt failed: {}", e),
                }
            }
            None => warn!("[Vault] Slot {} is NULL", slot),
        }
    }

    pub fn decrypt_slot(&mut self, id: u64, slot: usize) {
        match load_from_vault(id) {
            Ok(mut entry) => {
                entry.encrypted = false;
                info!("[Vault] Decrypted id={} → slot {}", id, slot);
                self.static_store.write(slot, entry);
            }
            Err(e) => warn!("[Vault] Decrypt failed: {}", e),
        }
    }
}