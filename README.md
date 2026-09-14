# Codex account profiles

Save and switch named local Codex CLI authentication profiles.
Rust port of `codex_accounts.py`.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/zyxwvutsrqponml/codex-accounts-rs/main/install.sh | sh
```

The installer downloads the latest release for Linux or macOS, verifies its
SHA-256 checksum, and places `codex-accounts` in `~/.local/bin`. Ensure that
directory is on your `PATH`.

Pin a version or change the install location:

```sh
CODEX_ACCOUNTS_VERSION=v0.1.0 CODEX_ACCOUNTS_INSTALL_DIR=~/.local/bin sh install.sh
```

Windows users: download `codex-accounts-x86_64-pc-windows-msvc.zip` from the
[latest release](https://github.com/zyxwvutsrqponml/codex-accounts-rs/releases/latest)
and add `codex-accounts.exe` to your `PATH`.

## Usage

This is the Rust port of `scripts/codex_accounts.py`. It manages named local
Codex CLI authentication profiles while keeping the same layout:

```text
$CODEX_HOME/auth.json
$CODEX_HOME/account-profiles/<name>/auth.json
```

Build from source with Cargo:

```bash
cargo build --release
./target/release/codex-accounts save personal
./target/release/codex-accounts save work
./target/release/codex-accounts list
./target/release/codex-accounts use personal
```

The default home is `CODEX_HOME`, then `~/.codex`. The tool never prints
credential contents, restricts profile directories/files on Unix, validates
profile names, and uses an atomic replacement for `auth.json`. Start a new
Codex process after switching. This tool is for file-backed `auth.json`
credentials; an OS-keychain-only login may not be copyable as a file.

## Releases

Push a version tag such as `v0.1.0` to publish a GitHub release. The release
workflow builds checked binaries for Linux (x86_64 and ARM64), macOS (Intel and
Apple Silicon), and Windows (x86_64).
