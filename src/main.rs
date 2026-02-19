mod clipboard;
mod config;
mod daemon;
mod engine;
mod hotkey;
mod storage;

use crate::config::{set_mode, ClipMode};
use crate::engine::Engine;
use crate::hotkey::HotkeyAction;
use crate::storage::{disk, RamStore};
use clap::{Parser, Subcommand};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "clipwallet",
    version = "0.1.0",
    about   = "A persistent, encrypted clipboard manager for macOS",
    long_about = "
ClipWallet — Multi-slot clipboard manager with RAM/disk/encrypted storage.

MODES:
  Static  — 9 named slots (Cmd+Opt+C/V/X + digit 1-9)
  Dynamic — Recency-ordered ring of up to 50 entries

Run 'clipwallet run' to start the background service.
Run 'clipwallet install' to register it as a login daemon.
Run 'clipwallet mode static' or 'clipwallet mode dynamic' to switch modes.
"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the ClipWallet background service (foreground)
    Run,

    /// Install ClipWallet as a launchd login agent
    Install,

    /// Uninstall the launchd agent
    Uninstall {
        /// Also wipe all stored data and encryption key
        #[arg(long, default_value_t = false)]
        purge: bool,
    },

    /// Show daemon and storage status
    Status,

    /// Set clipboard mode: static or dynamic
    Mode {
        #[arg(value_enum)]
        mode: CliMode,
    },

    /// List all encrypted vault entries
    VaultList,

    /// Delete an encrypted vault entry by ID
    VaultDelete {
        #[arg(help = "Entry ID to delete from vault")]
        id: u64,
    },

    /// Rotate the vault encryption key
    VaultRotate,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum CliMode {
    Static,
    Dynamic,
}

// ── Entry Point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run => run_service().await?,

        Commands::Install => daemon::plist::install()?,

        Commands::Mode { mode } => {
            let m = match mode {
                CliMode::Static  => ClipMode::Static,
                CliMode::Dynamic => ClipMode::Dynamic,
            };
            set_mode(m)?;
        }

        Commands::Uninstall { purge } => {
            daemon::plist::uninstall()?;

            if purge {
                storage::delete_key()?;
                daemon::status::wipe_all_data()?;
                println!("ClipWallet fully purged ✓");
            } else {
                println!("Remove encryption key from Keychain? [y/N]");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("y") {
                    if let Err(e) = storage::delete_key() {
                        eprintln!("Warning: Could not remove key: {}", e);
                    }
                }

                println!("Also delete all stored clipboard data? [y/N]");
                let mut input2 = String::new();
                std::io::stdin().read_line(&mut input2)?;
                if input2.trim().eq_ignore_ascii_case("y") {
                    daemon::status::wipe_all_data()?;
                }
            }
        }

        Commands::Status => {
            daemon::plist::status();
            daemon::status::print_full_status();
        }

        Commands::VaultList => {
            let ids = storage::list_vault_ids();
            if ids.is_empty() {
                println!("Vault is empty.");
            } else {
                println!("── Vault Entries ────────────────────────────────");
                for id in &ids {
                    println!("  id = {}", id);
                }
                println!("  Total: {}", ids.len());
                println!("─────────────────────────────────────────────────");
            }
        }

        Commands::VaultDelete { id } => {
            storage::delete_from_vault(id)?;
            println!("Vault entry {} deleted ✓", id);
        }

        Commands::VaultRotate => rotate_vault_key()?,
    }

    Ok(())
}

// ── Service Loop ──────────────────────────────────────────────────────────────

async fn run_service() -> anyhow::Result<()> {
    // ── Load mode from persistent config ─────────────────────────────
    let cfg         = config::load();
    let mode_static = cfg.mode == ClipMode::Static;
    info!(
        "ClipWallet v0.1.0 starting in {} mode...",
        if mode_static { "STATIC" } else { "DYNAMIC" }
    );

    // ── Shared RAM store ──────────────────────────────────────────────
    let ram = Arc::new(RwLock::new(RamStore::new()));
    {
        let mut w = ram.write().unwrap();
        disk::cleanup_tmp_files();
        if let Err(e) = disk::load(&mut w) {
            error!("Disk load failed: {}", e);
        }
    }
    info!("Disk state loaded into RAM ✓");

    // ── Shared tokio channel — all tasks send actions here ────────────
    let (tx, mut rx) = mpsc::unbounded_channel::<HotkeyAction>();

    // ── Task 1: CGEventTap key grabber ────────────────────────────────
    // CGEventTap intercepts keys at the macOS HID layer and can suppress
    // them (return None) before they reach any other application.
    // It must run on its own OS thread with its own CFRunLoop — it cannot
    // live inside tokio's async runtime.
    //
    // We bridge std::sync::mpsc → tokio::mpsc via a lightweight relay task.

    let (tap_tx, tap_rx) = std::sync::mpsc::sync_channel::<HotkeyAction>(64);

    // Relay task: forwards actions from the CGEventTap thread into tokio
    let tx_bridge = tx.clone();
    tokio::task::spawn_blocking(move || {
        while let Ok(action) = tap_rx.recv() {
            let _ = tx_bridge.send(action);
        }
    });

    // CGEventTap thread — blocks forever running CFRunLoop
    std::thread::spawn(move || {
        hotkey::grabber::start_event_tap(tap_tx, mode_static);
    });

    // ── Task 2: Engine event loop ─────────────────────────────────────
    let ram_engine  = Arc::clone(&ram);
    let mut engine  = Engine::new(ram_engine)?;
    let engine_handle = tokio::spawn(async move {
        while let Some(action) = rx.recv().await {
            engine.handle(action);
        }
    });

    // ── Task 3: Periodic flush (every 60 seconds) ─────────────────────
    let ram_flush = Arc::clone(&ram);
    let flush_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let mut w = ram_flush.write().unwrap();
            if let Err(e) = disk::flush(&mut w) {
                error!("Periodic flush failed: {}", e);
            }
        }
    });

    // ── Task 4: Signal handler (SIGTERM / Ctrl+C → clean shutdown) ────
    let ram_signal = Arc::clone(&ram);
    ctrlc::set_handler(move || {
        info!("Shutdown signal — flushing to disk...");
        let mut w = ram_signal.write().unwrap();
        let _ = disk::flush(&mut w);
        info!("ClipWallet stopped cleanly ✓");
        std::process::exit(0);
    })
    .expect("Cannot set signal handler");

    // ── Task 5: Cursor timeout watchdog ──────────────────────────────
    // Fires CursorReset if the dynamic ring cursor hasn't moved in
    // CURSOR_TIMEOUT_SECS seconds, snapping it back to the most recent entry.
    let ram_cursor = Arc::clone(&ram);
    let tx_cursor  = tx.clone();
    let cursor_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            if ram_cursor.read().unwrap().cursor_timed_out() {
                let _ = tx_cursor.send(HotkeyAction::CursorReset);
            }
        }
    });

    info!(
        "ClipWallet running ✓  |  Mode: {}  |  Listening for hotkeys...",
        if mode_static { "STATIC" } else { "DYNAMIC" }
    );
    info!(
        "Tip: System Settings → Privacy & Security → Accessibility → add clipwallet"
    );

    let _ = tokio::join!(engine_handle, flush_handle, cursor_handle);
    Ok(())
}

// ── Vault Key Rotation ────────────────────────────────────────────────────────

fn rotate_vault_key() -> anyhow::Result<()> {
    use storage::encrypted::{
        decrypt_entry, delete_key, get_or_create_key, save_to_vault, vault_dir,
    };

    println!("Rotating encryption key...");

    let dir = vault_dir();
    if !dir.exists() {
        println!("No vault entries found — nothing to rotate.");
        return Ok(());
    }

    let ids = storage::list_vault_ids();
    let mut entries = Vec::new();

    for id in &ids {
        let path  = dir.join(format!("{}.vlt", id));
        let bytes = std::fs::read(&path)?;
        match decrypt_entry(&bytes) {
            Ok(e)  => entries.push(e),
            Err(e) => eprintln!("Warning: Could not decrypt id={}: {}", id, e),
        }
    }

    // Delete old key → get_or_create_key generates and stores a fresh one
    delete_key()?;
    get_or_create_key()?;

    for entry in &entries {
        save_to_vault(entry)?;
    }

    println!("Key rotated — {} entries re-encrypted ✓", entries.len());
    Ok(())
}