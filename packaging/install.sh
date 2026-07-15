#!/bin/sh
# miniVE Linux installer.
#
#   curl -fsSL https://raw.githubusercontent.com/SahilSidhu7/miniVE/master/packaging/install.sh | sh
#
# Debian/Ubuntu: downloads and installs the latest .deb (apt handles updates
# of dependencies). Everything else: downloads the AppImage to ~/.local/bin.
set -e

REPO="SahilSidhu7/miniVE"
API="https://api.github.com/repos/$REPO/releases/latest"

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required." >&2; exit 1; }

case "$(uname -m)" in
  x86_64) DEB_ARCH="amd64" ;;
  aarch64) DEB_ARCH="arm64" ;;
  *) echo "Error: unsupported architecture $(uname -m)." >&2; exit 1 ;;
esac

echo "Looking up the latest miniVE release..."
ASSETS=$(curl -fsSL "$API" | grep -o '"browser_download_url": *"[^"]*"' | cut -d'"' -f4)
[ -n "$ASSETS" ] || { echo "Error: could not read the latest release from GitHub." >&2; exit 1; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if command -v dpkg >/dev/null 2>&1; then
  URL=$(echo "$ASSETS" | grep "_${DEB_ARCH}.deb$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no .deb asset for $DEB_ARCH in the latest release." >&2; exit 1; }
  echo "Downloading $(basename "$URL")..."
  curl -fL -o "$TMP/minive.deb" "$URL"
  echo "Installing (needs sudo)..."
  SUDO=""
  [ "$(id -u)" -ne 0 ] && SUDO="sudo"
  $SUDO apt-get install -y "$TMP/minive.deb" 2>/dev/null || $SUDO dpkg -i "$TMP/minive.deb"
  echo "Done. Launch 'minive-app' from your app menu, or 'minive' in a terminal."
else
  URL=$(echo "$ASSETS" | grep -i "\.AppImage$" | head -n1)
  [ -n "$URL" ] || { echo "Error: no AppImage asset in the latest release." >&2; exit 1; }
  BIN_DIR="${XDG_DATA_HOME:-$HOME/.local}/bin"
  mkdir -p "$BIN_DIR"
  echo "Downloading $(basename "$URL") to $BIN_DIR/minive-app..."
  curl -fL -o "$BIN_DIR/minive-app" "$URL"
  chmod +x "$BIN_DIR/minive-app"
  echo "Done. Run: $BIN_DIR/minive-app"
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Note: add $BIN_DIR to your PATH to run it as 'minive-app'." ;;
  esac
fi
