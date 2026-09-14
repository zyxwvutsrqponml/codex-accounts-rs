#!/bin/sh
# Install the newest codex-accounts release on Linux or macOS.
set -eu

REPOSITORY="${CODEX_ACCOUNTS_REPOSITORY:-zyxwvutsrqponml/codex-accounts-rs}"
VERSION="${CODEX_ACCOUNTS_VERSION:-latest}"
INSTALL_DIR="${CODEX_ACCOUNTS_INSTALL_DIR:-$HOME/.local/bin}"
BINARY="codex-accounts"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) platform="unknown-linux-gnu" ;;
  Darwin) platform="apple-darwin" ;;
  *) echo "Error: unsupported operating system: $os" >&2; exit 1 ;;
esac

case "$arch" in
  x86_64|amd64) target="x86_64-$platform" ;;
  aarch64|arm64) target="aarch64-$platform" ;;
  *) echo "Error: unsupported architecture: $arch" >&2; exit 1 ;;
esac

if [ "$target" = "x86_64-apple-darwin" ]; then
  echo "Error: Intel macOS builds are no longer published. Supported targets: Linux (x86_64, ARM64), macOS (Apple Silicon), Windows (x86_64, manual download)." >&2
  exit 1
fi

if [ "$VERSION" = "latest" ]; then
  base_url="https://github.com/$REPOSITORY/releases/latest/download"
else
  base_url="https://github.com/$REPOSITORY/releases/download/$VERSION"
fi

archive="$BINARY-$target.tar.gz"
temporary_dir="$(mktemp -d)"
cleanup() { rm -rf "$temporary_dir"; }
trap cleanup EXIT HUP INT TERM

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$1" -O "$2"
  else
    echo "Error: curl or wget is required." >&2
    exit 1
  fi
}

download "$base_url/$archive" "$temporary_dir/$archive"
download "$base_url/checksums.txt" "$temporary_dir/checksums.txt"

expected="$(awk -v file="$archive" '$2 == file { print $1 }' "$temporary_dir/checksums.txt")"
if [ -z "$expected" ]; then
  echo "Error: checksum for $archive was not found." >&2
  exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$temporary_dir/$archive" | awk '{print $1}')"
else
  actual="$(shasum -a 256 "$temporary_dir/$archive" | awk '{print $1}')"
fi
if [ "$actual" != "$expected" ]; then
  echo "Error: downloaded archive checksum does not match." >&2
  exit 1
fi

tar -xzf "$temporary_dir/$archive" -C "$temporary_dir"
mkdir -p "$INSTALL_DIR"
install -m 755 "$temporary_dir/$BINARY" "$INSTALL_DIR/$BINARY"
echo "Installed $BINARY to $INSTALL_DIR/$BINARY"
