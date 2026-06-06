#!/bin/sh
# oaifai installer — downloads the prebuilt binary and installs it system-wide.
#   curl -fsSL https://raw.githubusercontent.com/p32929/oaifai/master/install.sh | sh
set -e

REPO="p32929/oaifai"
DEST="/usr/local/bin/oaifai"
BIN_URL="https://github.com/${REPO}/releases/latest/download/oaifai"

os="$(uname -s)"
arch="$(uname -m)"
if [ "$os" != "Linux" ] || [ "$arch" != "x86_64" ]; then
  echo "oaifai's prebuilt binary is Linux x86_64 only (you have: $os $arch)."
  echo "Build from source instead:  git clone https://github.com/${REPO} && cd oaifai && cargo build --release"
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; else SUDO=""; fi

echo "Downloading oaifai..."
tmp="$(mktemp)"
curl -fsSL "$BIN_URL" -o "$tmp"
chmod +x "$tmp"

echo "Installing to $DEST (may ask for your password)..."
$SUDO install -m755 "$tmp" "$DEST"
rm -f "$tmp"

echo ""
echo "Done. Run it with:"
echo "    oaifai"
