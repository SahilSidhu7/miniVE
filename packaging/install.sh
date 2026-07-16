#!/bin/sh
# miniVE installer — macOS, Linux, WSL.
#
#   curl -fsSL https://sahilsidhu7.github.io/minive-landing/install.sh | bash
#
# macOS: Homebrew cask if brew is available, otherwise the .dmg.
# Debian/Ubuntu (incl. WSL): .deb via apt/dpkg.
# Fedora/RHEL: .rpm via dnf/rpm.  Anything else: AppImage to ~/.local/bin.
set -e

REPO="SahilSidhu7/miniVE"
API="https://api.github.com/repos/$REPO/releases/latest"

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required." >&2; exit 1; }

assets() {
  curl -fsSL "$API" | grep -o '"browser_download_url": *"[^"]*"' | cut -d'"' -f4
}

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

OS=$(uname -s)
ARCH=$(uname -m)

SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi

# The minive CLI ships as a separate release asset (the app bundles only contain minive-app).
install_cli() {
  case "$OS-$ARCH" in
    Darwin-arm64)  CLI_ASSET="minive-cli-macos-aarch64.tar.gz" ;;
    Darwin-x86_64) CLI_ASSET="minive-cli-macos-x86_64.tar.gz" ;;
    Linux-x86_64)  CLI_ASSET="minive-cli-linux-x86_64.tar.gz" ;;
    *) return 0 ;;
  esac
  CLI_URL=$(assets | grep "$CLI_ASSET\$" | head -n1)
  if [ -z "$CLI_URL" ]; then
    echo "Note: this release has no prebuilt CLI for $OS/$ARCH; see the README for building it from source."
    return 0
  fi
  echo "Installing the minive CLI..."
  curl -fL -o "$TMP/$CLI_ASSET" "$CLI_URL"
  tar -xzf "$TMP/$CLI_ASSET" -C "$TMP"
  if [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
    install -m 755 "$TMP/minive" /usr/local/bin/minive
    echo "CLI installed: /usr/local/bin/minive"
  elif command -v sudo >/dev/null 2>&1 || [ -z "$SUDO" ]; then
    $SUDO mkdir -p /usr/local/bin
    $SUDO install -m 755 "$TMP/minive" /usr/local/bin/minive
    echo "CLI installed: /usr/local/bin/minive"
  else
    mkdir -p "$HOME/.local/bin"
    install -m 755 "$TMP/minive" "$HOME/.local/bin/minive"
    echo "CLI installed: $HOME/.local/bin/minive (make sure it is on your PATH)"
  fi
}

if [ "$OS" = "Darwin" ]; then
  if command -v brew >/dev/null 2>&1; then
    echo "Installing via Homebrew..."
    brew install --cask sahilsidhu7/tap/minive
    install_cli
    exit 0
  fi
  case "$ARCH" in
    arm64) PAT="_aarch64.dmg" ;;
    x86_64) PAT="_x64.dmg" ;;
    *) echo "Error: unsupported macOS architecture $ARCH." >&2; exit 1 ;;
  esac
  URL=$(assets | grep "$PAT$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no $PAT asset in the latest release." >&2; exit 1; }
  echo "Downloading $(basename "$URL")..."
  curl -fL -o "$TMP/minive.dmg" "$URL"
  echo "Installing to /Applications..."
  VOL=$(hdiutil attach "$TMP/minive.dmg" -nobrowse | grep -o '/Volumes/.*' | head -n1)
  cp -R "$VOL"/*.app /Applications/
  hdiutil detach "$VOL" >/dev/null
  install_cli
  echo "Done. Launch miniVE from Applications."
  exit 0
fi

# Linux / WSL from here.
case "$ARCH" in
  x86_64) DEB_ARCH="amd64"; RPM_ARCH="x86_64" ;;
  aarch64) DEB_ARCH="arm64"; RPM_ARCH="aarch64" ;;
  *) echo "Error: unsupported architecture $ARCH." >&2; exit 1 ;;
esac

ALL=$(assets)
[ -n "$ALL" ] || { echo "Error: could not read the latest release from GitHub." >&2; exit 1; }

if command -v dpkg >/dev/null 2>&1; then
  URL=$(echo "$ALL" | grep "_${DEB_ARCH}.deb$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no .deb asset for $DEB_ARCH in the latest release." >&2; exit 1; }
  echo "Downloading $(basename "$URL")..."
  curl -fL -o "$TMP/minive.deb" "$URL"
  echo "Installing (needs sudo)..."
  $SUDO apt-get install -y "$TMP/minive.deb" 2>/dev/null || $SUDO dpkg -i "$TMP/minive.deb"
  install_cli
  echo "Done. Launch 'minive-app' from your app menu."
elif command -v rpm >/dev/null 2>&1; then
  URL=$(echo "$ALL" | grep "\.${RPM_ARCH}.rpm$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no .rpm asset for $RPM_ARCH in the latest release." >&2; exit 1; }
  echo "Downloading $(basename "$URL")..."
  curl -fL -o "$TMP/minive.rpm" "$URL"
  echo "Installing (needs sudo)..."
  if command -v dnf >/dev/null 2>&1; then $SUDO dnf install -y "$TMP/minive.rpm"; else $SUDO rpm -i "$TMP/minive.rpm"; fi
  install_cli
  echo "Done. Launch 'minive-app' from your app menu."
else
  URL=$(echo "$ALL" | grep -i "\.AppImage$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no AppImage asset in the latest release." >&2; exit 1; }
  BIN_DIR="$HOME/.local/bin"
  mkdir -p "$BIN_DIR"
  echo "Downloading $(basename "$URL") to $BIN_DIR/minive-app..."
  curl -fL -o "$BIN_DIR/minive-app" "$URL"
  chmod +x "$BIN_DIR/minive-app"
  install_cli
  echo "Done. Run: $BIN_DIR/minive-app"
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Note: add $BIN_DIR to your PATH to run it as 'minive-app'." ;;
  esac
fi
