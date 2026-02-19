ClipWallet v0.1.0 — Persistent Clipboard Manager for macOS
================================================================

INSTALL (double-click method):
  1. Open Terminal
  2. cd into this folder
  3. Run: ./install.sh

INSTALL (one-line):
  curl -fsSL https://github.com/yourusername/clipwallet/releases/latest/download/install.sh | sh

AFTER INSTALL:
  Grant Accessibility access:
  System Settings → Privacy & Security → Accessibility → + → clipwallet

HOTKEYS:
  Cmd+Opt+C+[1-9]       Copy into static slot
  Cmd+Opt+V+[1-9]       Paste from static slot
  Cmd+Opt+X+[1-9]       Cut into static slot
  Cmd+Opt+Tab           Navigate forward (dynamic ring)
  Cmd+Opt+Shift+Tab     Navigate backward
  Cmd+Opt+Tab+Esc       Delete current entry
  Cmd+Opt+C             Dynamic copy (no digit)
  Cmd+Opt+X             Dynamic cut (no digit)

COMMANDS:
  clipwallet status
  clipwallet vault-list
  clipwallet vault-delete <id>
  clipwallet vault-rotate
  clipwallet uninstall
  clipwallet uninstall --purge

LOGS:
  tail -f ~/.clipwallet/logs/out.log
