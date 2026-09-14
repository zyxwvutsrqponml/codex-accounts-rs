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
CODEX_ACCOUNTS_VERSION=v0.2.0 CODEX_ACCOUNTS_INSTALL_DIR=~/.local/bin sh install.sh
```

The installer also sets up tab-completion for bash/zsh/fish.
Set `CODEX_ACCOUNTS_SKIP_COMPLETIONS=1` to skip that step.

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

### Commands

```text
codex-accounts [--codex-home PATH] [--no-color] [--plain] <command> ...

save NAME [--from AUTH_JSON] [--force]
use NAME
list [--plain]
remove|delete|rm NAME
new|restart|clear|logout-local
path
completion <bash|zsh|fish|powershell>
help
```

`delete` and `rm` are aliases of `remove`. `restart`, `clear`
(and `logout-local`) are aliases of `new`.

### Interface

Colors auto-disable when piped, with `NO_COLOR=1`, `CLICOLOR=0`,
`TERM=dumb`, or `--no-color`. All measurements are exact: table
columns are padded to the widest value so everything lines up.

```text
$ codex-accounts list
Saved profiles (2):
  * personal  active
    work

$ codex-accounts path
CODEX_HOME   /home/kali/.codex
Active auth  /home/kali/.codex/auth.json
Profiles     /home/kali/.codex/account-profiles

$ codex-accounts use personal
✓ Activated profile 'personal'.
Start a new Codex process to use it.
```

For scripts, use stable plain output:

```bash
codex-accounts --plain list
codex-accounts list --plain
# * personal
#   work
```

### Why does `codex logout` break my saved profiles?

`codex logout` revokes the token on the server side. Any copy you
previously made with `codex-accounts save` points to the same revoked
refresh token, so `codex-accounts use <old-profile>` stops working.
This is expected OAuth behavior — the tool cannot resurrect a revoked
token.

What to do when a saved token is dead:

```bash
codex login
codex-accounts save personal --force
```

To avoid killing saved profiles in the first place, never use
`codex logout` for switching. Use the local-only reset instead:

```bash
codex-accounts new        # or: restart / clear
codex login
codex-accounts save newprofile
```

`new` only deletes `$CODEX_HOME/auth.json` on disk. It never contacts
the server, so already-saved profiles stay valid.

### Tab-completion (type `a`, press `TAB` → `abcd`)

Profile names, subcommands, and flags complete in bash, zsh, fish, and
PowerShell end to end:

```bash
codex-accounts use a<TAB>        # completes to saved profile, e.g. abcd
codex-accounts remove w<TAB>     # completes saved profile for delete/remove/rm
codex-accounts completion <TAB>  # completes: bash zsh fish powershell
```

Setup (the `install.sh` installer already does the first three):

```bash
# bash (requires bash-completion)
codex-accounts completion bash > ~/.local/share/bash-completion/completions/codex-accounts

# zsh
mkdir -p ~/.zfunc
codex-accounts completion zsh > ~/.zfunc/_codex-accounts
# add to ~/.zshrc BEFORE `compinit`: fpath=(~/.zfunc $fpath)

# fish
mkdir -p ~/.config/fish/completions
codex-accounts completion fish > ~/.config/fish/completions/codex-accounts.fish

# PowerShell (add to $PROFILE)
codex-accounts completion powershell | Out-String | Invoke-Expression
```

Completion resolves profiles from `${CODEX_HOME:-~/.codex}/account-profiles/*/auth.json`
and honors `--codex-home` / `$CODEX_HOME`. Restart your shell after installing.

The default home is `CODEX_HOME`, then `~/.codex`. The tool never prints
credential contents, restricts profile directories/files on Unix, validates
profile names, and uses an atomic replacement for `auth.json`. Start a new
Codex process after switching. This tool is for file-backed `auth.json`
credentials; an OS-keychain-only login may not be copyable as a file.

## Releases

Push a version tag such as `v0.2.0` to publish a GitHub release. The release
workflow builds checked binaries for Linux (x86_64 and ARM64), macOS
(Apple Silicon), and Windows (x86_64).
