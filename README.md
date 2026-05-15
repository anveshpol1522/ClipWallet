<p align="center">
  <h1 align="center">ClipWallet</h1>
  <p align="center">
    <strong>A persistent, encrypted, multi-slot clipboard manager for macOS.</strong>
  </p>
  <p align="center">
    <a href="#installation">Install</a> •
    <a href="#hotkeys">Hotkeys</a> •
    <a href="#modes">Modes</a> •
    <a href="#commands">Commands</a> •
    <a href="#architecture">Architecture</a> •
    <a href="#contributing">Contributing</a>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square&logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/platform-macOS-blue?style=flat-square&logo=apple" alt="macOS">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License">
    <img src="https://img.shields.io/badge/version-0.1.0-purple?style=flat-square" alt="Version">
    <img src="https://img.shields.io/badge/GSSoC'25-Contributor%20Friendly-yellow?style=flat-square" alt="GSSoC">
  </p>
</p>

---

## The Problem

You use `Ctrl+C` / `Cmd+C` a million times a day. Every single time, your previous copy is **gone forever**. You're one paste away from losing that URL, that code snippet, that address you copied 30 seconds ago.

## The Solution

**ClipWallet** turns your clipboard into a **wallet** — multiple addressable memory slots that persist across reboots, survive crashes, and encrypt sensitive data automatically. No GUI, no menubar clutter. Pure keyboard-native power.

---

## Features

| Feature | Description |
|---------|-------------|
| 🗂️ **Multi-slot clipboard** | 9 static slots or a 50-entry dynamic ring buffer |
| 💾 **Persistent storage** | Clipboard history survives reboots and crashes |
| 🔐 **AES-256-GCM encryption** | Vault-encrypted entries with macOS Keychain key storage |
| ⌨️ **Keyboard-native** | No GUI — controlled entirely through hotkey chords |
| 🖼️ **Rich data types** | Supports text, RTF, images (PNG/JPEG), file paths, binary data |
| 🔄 **Two operating modes** | Static (addressed slots) or Dynamic (recency ring) |
| 🚀 **Background daemon** | Runs as a `launchd` login agent, auto-starts on boot |
| 📦 **Single binary** | Zero-dependency Rust binary — no Electron, no frameworks |
| 🛡️ **Crash-safe writes** | Atomic tmp+rename disk persistence |
| 📁 **Pointer-style files** | File paths stored, never file contents — no RAM bloat |

---

## Installation

### Prerequisites

- **macOS** 12.0+ (Monterey or later)
- **Rust** toolchain (for building from source)
- **Xcode Command Line Tools**

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify Xcode CLI tools
xcode-select --version
# If not installed:
xcode-select --install
```

### Build & Install (Manual — Step by Step)

```bash
# 1. Navigate to project
cd ~/Documents/ClipWallet

# 2. Build release binary
cargo build --release

# 3. Install binary + code-sign it
sudo cp target/release/clipwallet /usr/local/bin/clipwallet
sudo codesign --sign - --force /usr/local/bin/clipwallet

# 4. Choose your mode (one time only — pick one)
clipwallet change mode

# 5. Install as login daemon (auto-starts on boot)
clipwallet install

# 6. Verify it's running
clipwallet status
```

### One-Line Install (from Release)

```bash
curl -fsSL https://github.com/shaaravraghu/ClipWallet/releases/latest/download/install.sh | sh
```

### Post-Install: Grant Accessibility Permission

ClipWallet requires Accessibility access to intercept hotkeys:

```
System Settings → Privacy & Security → Accessibility → + → clipwallet
```

> **Note:** Without this permission, hotkey interception will not work.

---

## Developer Workflow

### Watch Live Logs While Testing Hotkeys

```bash
tail -f ~/.clipwallet/logs/out.log
```

### Run in Foreground with Full Debug Output

Useful when you want real-time structured logs without running as a daemon:

```bash
RUST_LOG=debug clipwallet run
```

### Stop, Rebuild, and Restart After a Code Change

```bash
clipwallet uninstall && \
cargo build --release && \
sudo cp target/release/clipwallet /usr/local/bin/clipwallet && \
sudo codesign --sign - --force /usr/local/bin/clipwallet && \
clipwallet install
```

> Run this as a single command after any source change to get a clean re-install cycle.

### Quick Reinstall (no rebuild)

If you've only changed config or reinstalling the daemon:

```bash
clipwallet uninstall && clipwallet install
```

---

## Hotkeys

### Static Mode — Addressed Slots (1-9)

| Hotkey | Action |
|--------|--------|
| `Cmd + Opt + C + [1-9]` | **Copy** selection into slot N |
| `Cmd + Opt + V + [1-9]` | **Paste** from slot N |
| `Cmd + Opt + X + [1-9]` | **Cut** selection into slot N |
| `Cmd + Opt + Tab` | **Navigate forward** through occupied slots |
| `Cmd + Opt + Shift + Tab` | **Navigate backward** through occupied slots |
| `Cmd + Opt + Tab + Esc` | **Delete** entry at current cursor position |

### Dynamic Mode — Recency Ring

| Hotkey | Action |
|--------|--------|
| `Cmd + Opt + C` | **Copy** selection → pushes to ring front |
| `Cmd + Opt + X` | **Cut** selection → pushes to ring front |
| `Cmd + Opt + V` | **Paste** entry at current cursor position |
| `Cmd + Opt + Tab` | **Navigate forward** (→ older entries) |
| `Cmd + Opt + Shift + Tab` | **Navigate backward** (→ newer entries) |
| `Cmd + Opt + Tab + Esc` | **Delete** entry at current cursor position |

> In both modes, navigating via Tab syncs the selected entry to the system clipboard. Press `Cmd+V` to paste it anywhere.

---

## Modes

### Static Mode

Nine independent, directly addressed slots. Think of them as numbered pockets:

```
Slot 1: [URL]     Slot 4: [empty]   Slot 7: [image]
Slot 2: [code]    Slot 5: [address]  Slot 8: [empty]
Slot 3: [empty]   Slot 6: [email]   Slot 9: [RTF doc]
```

Copy to slot 3 with `Cmd+Opt+C+3`, paste it anytime with `Cmd+Opt+V+3`.

### Dynamic Mode

A recency-ordered ring buffer (up to 50 entries). Every copy pushes to the front. Tab cycles through history:

```
[newest] → [2nd] → [3rd] → ... → [oldest]
    ▲                                  │
    └──────────────────────────────────┘
```

After 10 seconds of idle, the cursor auto-resets to the most recent entry.

### Switching Modes

```bash
clipwallet change mode
```

---

## Commands

| Command | Description |
|---------|-------------|
| `clipwallet run` | Start the service (foreground) |
| `clipwallet install` | Register as launchd login agent |
| `clipwallet uninstall` | Remove the launchd agent |
| `clipwallet uninstall --purge` | Remove agent + wipe all data + Keychain key |
| `clipwallet status` | Show daemon status, storage stats |
| `clipwallet change mode` | Interactively switch Static ↔ Dynamic |
| `clipwallet remove encryption` | Wipe vault + remove Keychain key |
| `clipwallet clear memory` | Erase all clipboard history from disk |
| `clipwallet vault-list` | List all encrypted vault entry IDs |
| `clipwallet vault-delete <id>` | Delete a specific vault entry |
| `clipwallet vault-rotate` | Rotate the encryption key (re-encrypts all entries) |

---

## Architecture

ClipWallet is built as a layered system with three storage tiers:

```
┌──────────────────────────────────────┐
│           Hotkey Layer               │
│  CGEventTap → ChordDetector → Engine│
├──────────────────────────────────────┤
│           Clipboard Layer            │
│  arboard + NSPasteboard (Obj-C FFI) │
├──────────────────────────────────────┤
│           Storage Layer              │
│  RAM → Disk (MessagePack) → Vault   │
│       (AES-256-GCM + Keychain)      │
└──────────────────────────────────────┘
```

**5 concurrent tasks** power the service:
1. **CGEventTap grabber** — intercepts and suppresses hotkeys (dedicated OS thread)
2. **Channel bridge** — bridges `std::sync` → `tokio` channels
3. **Engine event loop** — dispatches hotkey actions to clipboard operations
4. **Periodic flush** — writes dirty RAM to disk every 60 seconds
5. **Cursor timeout watchdog** — auto-resets dynamic cursor after 10s idle

For the full technical deep-dive, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Storage & Security

### Storage Locations

| Path | Contents |
|------|----------|
| `~/.clipwallet/config.toml` | Mode configuration |
| `~/.clipwallet/store/` | MessagePack clipboard persistence |
| `~/.clipwallet/vault/` | AES-256-GCM encrypted entries |
| `~/.clipwallet/logs/` | Daemon stdout/stderr logs |

### Encryption

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key storage:** macOS Keychain (`com.clipwallet.vault`)
- **Auto-generated:** Key created on first run if absent
- **Key rotation:** `clipwallet vault-rotate` re-encrypts all entries with a new key
- **Format:** `[ 12-byte nonce ‖ ciphertext ]` per `.vlt` file

---

## Supported Data Types

| Type | How It's Stored |
|------|----------------|
| Plain text | UTF-8 string, stored inline |
| Rich text (RTF) | Raw RTF bytes via NSPasteboard |
| Images (PNG/JPEG) | Normalised to PNG, stored as bytes |
| File paths | Pointer-style — paths only, never file contents |
| Binary data | Raw bytes with magic-byte MIME detection |

---

## Logs

```bash
# Watch live logs
tail -f ~/.clipwallet/logs/out.log

# Check errors
cat ~/.clipwallet/logs/err.log
```

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (Edition 2021) |
| Async Runtime | Tokio |
| Key Interception | Core Graphics CGEventTap |
| Clipboard Access | arboard + NSPasteboard (Obj-C FFI) |
| Serialisation | MessagePack (rmp-serde) |
| Encryption | AES-256-GCM (aes-gcm crate) |
| Key Storage | macOS Keychain (keyring crate) |
| CLI | clap v4 |
| Logging | tracing + tracing-subscriber |
| Daemon | macOS launchd (plist) |

---

## Contributing

We welcome contributions! ClipWallet is part of **GirlScript Summer of Code (GSSoC) 2025**.

### How to Contribute

1. **Fork** the repository
2. **Create a branch** for your feature/fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** — follow the existing code style
4. **Test locally:**
   ```bash
   cargo build
   cargo run -- run
   ```
5. **Commit** with a descriptive message:
   ```bash
   git commit -m "feat: add XYZ functionality"
   ```
6. **Push** and open a **Pull Request**

### Contribution Guidelines

- Follow Rust conventions (`cargo fmt`, `cargo clippy`)
- Keep PRs focused — one feature or fix per PR
- Add comments for non-obvious logic
- Update documentation if your change affects user-facing behaviour
- Reference the issue number in your PR description

### Good First Issues

Look for issues tagged with `good first issue` or `help wanted` in the [Issues](https://github.com/shaaravraghu/ClipWallet/issues) tab.

See [CONTRIBUTORS.md](CONTRIBUTORS.md) for a list of people who have contributed to this project.

---

## Uninstall

```bash
# Remove daemon only
clipwallet uninstall

# Full purge (removes all data + encryption key)
clipwallet uninstall --purge
```

---

## Roadmap

- [ ] Windows support
- [ ] Linux/Wayland support
- [ ] Clipboard history search
- [ ] Custom hotkey configuration
- [ ] Clipboard sharing across devices
- [ ] GUI overlay for visual slot selection
- [ ] Plugin system for custom clipboard transformations

---

## License

This project is open source and available under the [MIT License](LICENSE).

---

<p align="center">
  Built with ❤️ in Rust — because your clipboard deserves better.
</p>
