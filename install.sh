#!/usr/bin/env bash
set -euo pipefail

# ── ClipWallet Installer ───────────────────────────────────────────────────────
# Usage:
#   curl -fsSL https://your-host/install.sh | sh
#   OR: ./install.sh from the extracted zip

BINARY_NAME="clipwallet"
INSTALL_DIR="/usr/local/bin"
INSTALL_PATH="${INSTALL_DIR}/${BINARY_NAME}"
REPO_URL="https://github.com/yourusername/clipwallet"
RELEASE_URL="${REPO_URL}/releases/latest/download"

# ── Colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; NC='\033[0m'

info()    { echo -e "${BLUE}[ClipWallet]${NC} $1"; }
success() { echo -e "${GREEN}[ClipWallet]${NC} $1"; }
warn()    { echo -e "${YELLOW}[ClipWallet]${NC} $1"; }
error()   { echo -e "${RED}[ClipWallet]${NC} $1"; exit 1; }

# ── Detect Architecture ───────────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
    arm64)  BINARY_SUFFIX="aarch64-apple-darwin"  ;;
    x86_64) BINARY_SUFFIX="x86_64-apple-darwin"   ;;
    *)      error "Unsupported architecture: $ARCH" ;;
esac

info "Detected architecture: ${ARCH}"

# ── Check if running from extracted zip ───────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCAL_BINARY="${SCRIPT_DIR}/${BINARY_NAME}"

if [ -f "$LOCAL_BINARY" ]; then
    info "Found local binary — installing from zip..."
    BINARY_SOURCE="$LOCAL_BINARY"
else
    # ── Download from GitHub Releases ─────────────────────────────────
    info "Downloading ClipWallet for ${BINARY_SUFFIX}..."
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    DOWNLOAD_URL="${RELEASE_URL}/${BINARY_NAME}-${BINARY_SUFFIX}"
    curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${BINARY_NAME}" \
        || error "Download failed. Check your internet connection."
    BINARY_SOURCE="${TMP_DIR}/${BINARY_NAME}"
fi

# ── Install Binary ────────────────────────────────────────────────────────────
info "Installing to ${INSTALL_PATH}..."
sudo mkdir -p "$INSTALL_DIR"
sudo cp "$BINARY_SOURCE" "$INSTALL_PATH"
sudo chmod +x "$INSTALL_PATH"

# ── Code Sign ─────────────────────────────────────────────────────────────────
info "Signing binary (ad-hoc)..."
sudo xattr -d com.apple.quarantine "$INSTALL_PATH" 2>/dev/null || true
sudo codesign --sign - --force "$INSTALL_PATH" \
    || warn "Code signing failed — you may need to grant permission manually"

# ── Create Runtime Directories ────────────────────────────────────────────────
mkdir -p "$HOME/.clipwallet/logs"
mkdir -p "$HOME/.clipwallet/store"
mkdir -p "$HOME/.clipwallet/vault"

# ── Register launchd Agent ────────────────────────────────────────────────────
info "Registering login daemon..."
"$INSTALL_PATH" install

# ── Accessibility Permission Reminder ────────────────────────────────────────
echo ""
success "ClipWallet installed successfully ✓"
echo ""
warn "IMPORTANT: Grant Accessibility permission for hotkeys to work:"
echo "  System Settings → Privacy & Security → Accessibility"
echo "  Click + and add: ${INSTALL_PATH}"
echo ""
info "Useful commands:"
echo "  clipwallet status          — show running status"
echo "  clipwallet vault-list      — list encrypted entries"
echo "  clipwallet uninstall       — remove daemon"
echo "  clipwallet uninstall --purge — remove everything"
echo ""
success "Run 'tail -f ~/.clipwallet/logs/out.log' to watch live logs"