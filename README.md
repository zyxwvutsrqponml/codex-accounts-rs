# Codex Accounts

Save and switch named local Codex CLI authentication profiles.

Codex Accounts lets you save the current local login under a profile name and
switch between profiles without repeatedly managing the files by hand. It is
local-first, lightweight, and designed for predictable use from both terminals
and scripts.

## Highlights

- Save, activate, list, and remove named profiles.
- Switch profiles without contacting the authentication server.
- Atomic file replacement to avoid partially written credentials.
- Profile-name validation and protected credential files on Unix systems.
- Plain output for automation and built-in shell completion.
- Native release builds for Linux, macOS, and Windows.

## Installation

### Linux and macOS

The installer downloads the latest verified release and installs the binary to
`~/.local/bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/zyxwvutsrqponml/codex-accounts-rs/main/install.sh | sh
```

Make sure `~/.local/bin` is on your `PATH`. To install a specific version or
choose another directory:

```sh
CODEX_ACCOUNTS_VERSION=v0.2.0 \
CODEX_ACCOUNTS_INSTALL_DIR="$HOME/.local/bin" \
sh install.sh
```

The installer also configures Bash, Zsh, and Fish completion when supported.
Set `CODEX_ACCOUNTS_SKIP_COMPLETIONS=1` to skip completion setup.

### Windows

Download `codex-accounts-x86_64-pc-windows-msvc.zip` from the
[latest release](https://github.com/zyxwvutsrqponml/codex-accounts-rs/releases/latest),
extract `codex-accounts.exe`, and add its directory to `PATH`.

The release contains a native Windows executable. The Unix `install.sh` script
is not required on Windows.

## Quick start

After signing in with the Codex CLI, save the active login:

```text
codex-accounts save personal
```

Create another login and save it under a different name, then switch whenever
you need to:

```text
codex login
codex-accounts save work
codex-accounts list
codex-accounts use personal
```

Start a new Codex process after switching profiles.

## Commands

```text
codex-accounts [OPTIONS] <COMMAND>

Commands:
  save NAME [--from AUTH_JSON] [--force]  Save the active login
  use NAME                                Activate a saved profile
  list [--plain]                          List saved profiles
  remove NAME                             Delete a saved profile
  new                                     Clear the active login locally
  path                                    Show resolved storage paths
  completion <SHELL>                      Print a completion script
  help                                    Show help
```

Aliases are also available: `delete` and `rm` map to `remove`; `restart`,
`clear`, and `logout-local` map to `new`.

Global options:

```text
--codex-home PATH  Use a custom Codex data directory
--no-color         Disable colored output
--plain            Use stable, script-friendly output
```

Examples:

```sh
codex-accounts save personal
codex-accounts save work --from ~/backups/work-auth.json
codex-accounts use work
codex-accounts list --plain
codex-accounts remove work
codex-accounts path
```

## Storage locations

The directory is resolved in this order:

1. `--codex-home PATH`
2. `CODEX_HOME`
3. The platform's home directory followed by `.codex`

Profiles use the same layout as the Codex CLI:

```text
<CODEX_HOME>/auth.json
<CODEX_HOME>/account-profiles/<name>/auth.json
```

Typical defaults are:

```text
Linux/macOS: $HOME/.codex
Windows:     %USERPROFILE%\.codex
```

Use `codex-accounts path` to see the exact paths detected on your machine.

## Shell completion

Completion scripts are available for Bash, Zsh, Fish, and PowerShell.

```sh
# Bash
codex-accounts completion bash > ~/.local/share/bash-completion/completions/codex-accounts

# Zsh
mkdir -p ~/.zfunc
codex-accounts completion zsh > ~/.zfunc/_codex-accounts

# Fish
mkdir -p ~/.config/fish/completions
codex-accounts completion fish > ~/.config/fish/completions/codex-accounts.fish
```

For PowerShell, add this command to `$PROFILE`:

```powershell
codex-accounts completion powershell | Out-String | Invoke-Expression
```

Restart the shell after installing or changing completion files.

## Important authentication behavior

`codex-accounts new` only removes the active local `auth.json`; it does not
contact the server and does not affect saved profiles.

By contrast, `codex logout` revokes the refresh token on the server. A saved
profile containing that token will no longer work after logout. To refresh a
profile, sign in again and overwrite it:

```sh
codex login
codex-accounts save personal --force
```

Codex Accounts manages file-backed `auth.json` credentials. A login stored
only in an operating-system keychain may not be available as a copyable file.

## Build from source

To build from source, run:

```sh
cargo build --release
```

The binary will be written to `target/release/codex-accounts` (or
`codex-accounts.exe` on Windows).

Run the test suite with:

```sh
cargo test --locked
```

## Releases

Releases are published from version tags such as `v0.2.0`. Automated builds
produce verified archives for:

- Linux x86_64 and ARM64
- macOS Apple Silicon
- Windows x86_64

See the [release page](https://github.com/zyxwvutsrqponml/codex-accounts-rs/releases)
for downloads and checksums.

## License

MIT
