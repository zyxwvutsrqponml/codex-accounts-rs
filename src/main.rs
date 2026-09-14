use serde_json::Value;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_DIR: &str = "account-profiles";

type Result<T> = std::result::Result<T, String>;

// ---------- CLI UI: colors + perfect measurement, zero dependencies ----------

#[derive(Clone, Copy, Debug)]
struct Ui {
    color: bool,
    plain: bool,
}

impl Ui {
    fn new(no_color: bool, plain: bool) -> Self {
        Self {
            color: color_enabled(no_color),
            plain,
        }
    }

    fn test() -> Self {
        Self {
            color: false,
            plain: false,
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn green(&self, text: &str) -> String {
        self.paint("32", text)
    }

    fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }

    fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }

    fn tick(&self) -> String {
        self.green("✓")
    }
}

fn color_enabled(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if env::var("CLICOLOR")
        .map(|value| value == "0")
        .unwrap_or(false)
    {
        return false;
    }
    if env::var("TERM")
        .map(|value| value == "dumb")
        .unwrap_or(false)
    {
        return false;
    }
    io::stdout().is_terminal()
}

fn stderr_color_enabled() -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if env::var("TERM")
        .map(|value| value == "dumb")
        .unwrap_or(false)
    {
        return false;
    }
    io::stderr().is_terminal()
}

fn print_error(message: &str) {
    if stderr_color_enabled() {
        eprintln!("\x1b[31m✗ Error:\x1b[0m {message}");
    } else {
        eprintln!("Error: {message}");
    }
}

/// Visible width in columns. Profile names are ASCII (`[A-Za-z0-9._- Slo-`),
/// so char count equals terminal columns; no unicode-width dependency needed.
fn display_width(text: &str) -> usize {
    text.chars().count()
}

/// Pad with spaces so a column ends at exactly `width` columns.
fn pad_to(text: &str, width: usize) -> String {
    let current = display_width(text);
    if current >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - current))
    }
}

fn codex_home(override_path: Option<&str>) -> PathBuf {
    if let Some(path) = override_path {
        return expand_user(path);
    }
    if let Some(path) = env::var_os("CODEX_HOME") {
        let text = path.to_string_lossy();
        // Match Python's Path(...).expanduser(): allow "~" in $CODEX_HOME.
        if text == "~" || text.starts_with("~/") || text.starts_with("~\\") {
            return expand_user(&text);
        }
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("HOME") {
        return PathBuf::from(path).join(".codex");
    }
    if let Some(path) = env::var_os("USERPROFILE") {
        return PathBuf::from(path).join(".codex");
    }
    PathBuf::from(".codex")
}

fn expand_user(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
            return PathBuf::from(home);
        }
        return PathBuf::from(path);
    }
    if path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
            let suffix = path[2..].replace('\\', "/");
            return PathBuf::from(home).join(suffix);
        }
    }
    PathBuf::from(path)
}

fn profile_root(home: &Path) -> PathBuf {
    home.join(PROFILE_DIR)
}

fn active_auth(home: &Path) -> PathBuf {
    home.join("auth.json")
}

fn validate_name(name: &str) -> Result<()> {
    let valid_length = (1..=64).contains(&name.len());
    let valid_chars = name
        .chars()
        .enumerate()
        .all(|(index, ch)| ch.is_ascii_alphanumeric() || (index > 0 && ".-_".contains(ch)));
    if valid_length && valid_chars && name.as_bytes()[0].is_ascii_alphanumeric() {
        Ok(())
    } else {
        Err("Invalid profile name. Use 1-64 letters, numbers, '.', '_' or '-'.".into())
    }
}

fn profile_auth(home: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    Ok(profile_root(home).join(name).join("auth.json"))
}

#[cfg(unix)]
fn restrict_permissions(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

fn ensure_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
    restrict_permissions(path, 0o700)
        .map_err(|error| format!("Cannot restrict permissions on {}: {error}", path.display()))
}

fn read_auth(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Authentication file not found: {} ({error})", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!("Authentication path is not a regular file: {}", path.display()));
    }
    let data = fs::read(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    if data.is_empty() {
        return Err(format!("Authentication file is empty: {}", path.display()));
    }
    let value: Value = serde_json::from_slice(&data)
        .map_err(|error| format!("Authentication file is not valid JSON: {} ({error})", path.display()))?;
    if !value.is_object() {
        return Err(format!("Authentication file must contain a JSON object: {}", path.display()));
    }
    Ok(data)
}

fn unique_temp_path(parent: &Path, filename: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    parent.join(format!(".{filename}.{}.{}", process::id(), nanos))
}

fn atomic_copy(source: &Path, destination: &Path, overwrite: bool) -> Result<()> {
    let data = read_auth(source)?;
    let parent = destination
        .parent()
        .ok_or_else(|| "Destination has no parent directory".to_string())?;
    // Only restrict the grandparent when it is the profile root
    // (`.../account-profiles`). The Python version unconditionally chmods
    // `destination.parent.parent`, which would wrongly chmod $HOME's parent
    // when activating (`use` -> $CODEX_HOME/auth.json). See issue parity fix.
    if let Some(root) = parent.parent() {
        if root.file_name().is_some_and(|name| name == PROFILE_DIR) {
            ensure_private_dir(root)?;
        }
    }
    ensure_private_dir(parent)?;
    if destination.exists() && !overwrite {
        // Match Python: report the profile name, not the full path.
        let label = parent
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| destination.display().to_string());
        return Err(format!("Profile already exists: {label} (use --force)"));
    }
    if destination.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        return Err(format!("Refusing to replace symlink: {}", destination.display()));
    }

    // Retry on the (extremely unlikely) temp-name collision, mirroring
    // Python's mkstemp O_EXCL uniqueness guarantee.
    let (temporary, mut file) = {
        let mut attempts = 0;
        loop {
            let candidate = unique_temp_path(parent, "auth.json");
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(handle) => break (candidate, handle),
                Err(error)
                    if error.kind() == io::ErrorKind::AlreadyExists && attempts < 10 =>
                {
                    attempts += 1;
                    continue;
                }
                Err(error) => {
                    return Err(format!("Cannot create temporary auth file: {error}"));
                }
            }
        }
    };
    restrict_permissions(&temporary, 0o600)
        .map_err(|error| format!("Cannot restrict temporary auth file: {error}"))?;
    let write_result = (|| -> io::Result<()> {
        file.write_all(&data)?;
        file.sync_all()?;
        Ok(())
    })();
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Cannot write temporary auth file: {error}"));
    }
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Cannot activate {}: {error}", destination.display()));
    }
    restrict_permissions(destination, 0o600)
        .map_err(|error| format!("Profile was saved, but permissions could not be restricted on {}: {error}", destination.display()))?;
    // Match Python's post-replace `chmod 0700` on the containing directory.
    restrict_permissions(parent, 0o700)
        .map_err(|error| format!("Profile was saved, but permissions could not be restricted on {}: {error}", parent.display()))?;
    Ok(())
}

fn save(home: &Path, name: &str, source: Option<&str>, force: bool, ui: &Ui) -> Result<()> {
    let source = source.map(expand_user).unwrap_or_else(|| active_auth(home));
    let destination = profile_auth(home, name)?;
    atomic_copy(&source, &destination, force)?;
    println!("{} Saved profile '{name}'.", ui.tick());
    Ok(())
}

fn use_profile(home: &Path, name: &str, ui: &Ui) -> Result<()> {
    let source = profile_auth(home, name)?;
    let destination = active_auth(home);
    atomic_copy(&source, &destination, true)?;
    println!("{} Activated profile '{name}'.", ui.tick());
    if !ui.plain {
        println!("{}", ui.dim("Start a new Codex process to use it."));
    }
    Ok(())
}

fn list_profiles(home: &Path, ui: &Ui) -> Result<()> {
    let root = profile_root(home);
    let active = active_auth(home);
    // Match Python: a symlinked active auth file counts as "no active digest"
    // (no `*` marker), it is never followed.
    let active_bytes = match fs::symlink_metadata(&active) {
        Ok(meta) if meta.file_type().is_file() => fs::read(&active).ok(),
        _ => None,
    };
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if ui.plain {
                println!("No saved profiles.");
            } else {
                println!("{}", ui.dim("No saved profiles."));
                println!(
                    "{}",
                    ui.dim("Run `codex login`, then `codex-accounts save <name>`.")
                );
            }
            return Ok(());
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", root.display())),
    };

    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("Cannot inspect profiles: {error}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if validate_name(&name).is_ok() && path.join("auth.json").is_file() {
            names.push(name);
        }
    }
    names.sort();
    if names.is_empty() {
        if ui.plain {
            println!("No saved profiles.");
        } else {
            println!("{}", ui.dim("No saved profiles."));
            println!(
                "{}",
                ui.dim("Run `codex login`, then `codex-accounts save <name>`.")
            );
        }
        return Ok(());
    }
    // Machine-readable fallback for scripts: `* name` / `  name`.
    if ui.plain {
        for name in &names {
            let profile = fs::read(profile_auth(home, name)?).ok();
            let marker = if active_bytes.is_some() && profile == active_bytes {
                '*'
            } else {
                ' '
            };
            println!("{marker} {name}");
        }
        return Ok(());
    }
    // Pretty table with perfect measurement: names padded to the same width
    // so the `active` column always starts at the same x position.
    let mut rows: Vec<(String, bool)> = Vec::with_capacity(names.len());
    for name in &names {
        let profile = fs::read(profile_auth(home, name)?).ok();
        let is_active = active_bytes.is_some() && profile == active_bytes;
        rows.push((name.clone(), is_active));
    }
    let name_width = rows
        .iter()
        .map(|(name, _)| display_width(name))
        .max()
        .unwrap_or(0);
    println!("{}", ui.bold(&format!("Saved profiles ({}):", rows.len())));
    for (name, is_active) in rows {
        let marker = if is_active {
            ui.green("*")
        } else {
            " ".to_string()
        };
        let padded = pad_to(&name, name_width);
        if is_active {
            println!("  {marker} {padded}  {}", ui.dim("active"));
        } else {
            println!("  {marker} {padded}");
        }
    }
    Ok(())
}

fn remove_profile(home: &Path, name: &str, ui: &Ui) -> Result<()> {
    let target = profile_auth(home, name)?;
    let metadata = fs::symlink_metadata(&target)
        .map_err(|_| format!("Profile not found: {name}"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("Refusing to remove symlink: {}", target.display()));
    }
    if !metadata.file_type().is_file() {
        return Err(format!("Refusing to remove non-file profile: {}", target.display()));
    }
    fs::remove_file(&target).map_err(|error| format!("Cannot remove {}: {error}", target.display()))?;
    if let Some(parent) = target.parent() {
        let _ = fs::remove_dir(parent);
    }
    println!("{} Removed profile '{name}'.", ui.tick());
    Ok(())
}

fn clear_active(home: &Path, ui: &Ui) -> Result<()> {
    let active = active_auth(home);
    match fs::symlink_metadata(&active) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if ui.plain {
                println!("No active login to clear.");
            } else {
                println!("{} {}", ui.dim("•"), ui.dim("No active login to clear."));
            }
            return Ok(());
        }
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!("Refusing to remove symlink: {}", active.display()));
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(format!("Refusing to remove non-file auth: {}", active.display()));
        }
        Ok(_) => {}
        Err(error) => {
            return Err(format!("Cannot inspect {}: {error}", active.display()));
        }
    }
    // Local-only removal: unlike `codex logout`, this never contacts the
    // server, so already-saved profile tokens stay valid.
    fs::remove_file(&active).map_err(|error| format!("Cannot remove {}: {error}", active.display()))?;
    println!("{} Cleared active login.", ui.tick());
    if !ui.plain {
        println!(
            "{}",
            ui.dim("Saved profiles are untouched. Run `codex login`, then `codex-accounts save <name> --force`.")
        );
    }
    Ok(())
}

fn print_completion(shell: &str) -> Result<()> {
    // Scripts live in `completions/` so install.sh / releases can ship the
    // same files; the binary embeds them for `completion <shell>`.
    let script = match shell.to_ascii_lowercase().as_str() {
        "bash" => include_str!("../completions/codex-accounts.bash"),
        "zsh" => include_str!("../completions/codex-accounts.zsh"),
        "fish" => include_str!("../completions/codex-accounts.fish"),
        "powershell" | "pwsh" | "ps" => include_str!("../completions/codex-accounts.ps1"),
        other => {
            return Err(format!(
                "Unknown shell: {other} (expected bash, zsh, fish, or powershell)"
            ));
        }
    };
    print!("{script}");
    Ok(())
}

fn show_paths(home: &Path, ui: &Ui) {
    // Perfect measurement: labels padded to the same width so all
    // values start at the same column.
    let labels = ["CODEX_HOME", "Active auth", "Profiles"];
    let width = labels
        .iter()
        .map(|label| display_width(label))
        .max()
        .unwrap_or(0);
    let values = [
        home.display().to_string(),
        active_auth(home).display().to_string(),
        profile_root(home).display().to_string(),
    ];
    for (label, value) in labels.iter().zip(values.iter()) {
        println!("{}  {value}", ui.bold(&pad_to(label, width)));
    }
}

fn usage() {
    // Aligned help: command column is measured, descriptions start together.
    let commands = [
        ("save NAME [--from AUTH_JSON] [--force]", "Save current login as a profile"),
        ("use NAME", "Activate a saved profile"),
        ("list [--plain]", "List saved profiles (* = active)"),
        ("remove|delete|rm NAME", "Remove a saved profile"),
        ("new|restart|clear", "Clear active login locally, keep profiles valid"),
        ("path", "Show resolved paths"),
        ("completion <shell>", "Print completion script (bash|zsh|fish|powershell)"),
        ("help", "Show this help"),
    ];
    let options = [
        ("--codex-home PATH", "Override CODEX_HOME"),
        ("--no-color", "Disable colors"),
        ("--plain", "Plain output for scripts (no colors, no table)"),
        ("-h, --help", "Show help"),
    ];
    let command_width = commands
        .iter()
        .map(|(command, _)| display_width(command))
        .max()
        .unwrap_or(0);
    let option_width = options
        .iter()
        .map(|(option, _)| display_width(option))
        .max()
        .unwrap_or(0);
    let bold = stderr_color_enabled();
    let header = |text: &str| {
        if bold {
            eprintln!("\x1b[1m{text}\x1b[0m");
        } else {
            eprintln!("{text}");
        }
    };
    eprintln!("Usage: codex-accounts [--codex-home PATH] [--no-color] [--plain] <command> ...");
    eprintln!();
    header("Commands:");
    for (command, description) in &commands {
        eprintln!("  {}  {description}", pad_to(command, command_width));
    }
    eprintln!();
    header("Options:");
    for (option, description) in &options {
        eprintln!("  {}  {description}", pad_to(option, option_width));
    }
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  codex-accounts save personal");
    eprintln!("  codex-accounts use personal");
    eprintln!("  codex-accounts list");
}

fn require_no_more_args(args: &[String], index: usize) -> Result<()> {
    if let Some(argument) = args.get(index) {
        Err(format!("Unexpected argument: {argument}"))
    } else {
        Ok(())
    }
}

fn run(args: &[String]) -> Result<()> {
    let mut index = 0;
    let mut owned_home: Option<String> = None;
    let mut no_color = false;
    let mut plain = false;
    // Global flags before the subcommand. Accept both `--codex-home PATH`
    // and `--codex-home=PATH` (argparse supports both, so we must too).
    while let Some(first) = args.get(index) {
        if first == "--codex-home" {
            owned_home = Some(
                args.get(index + 1)
                    .ok_or_else(|| "--codex-home requires a path".to_string())?
                    .clone(),
            );
            index += 2;
        } else if let Some(value) = first.strip_prefix("--codex-home=") {
            if value.is_empty() {
                return Err("--codex-home requires a path".to_string());
            }
            owned_home = Some(value.to_string());
            index += 1;
        } else if first == "--no-color" {
            no_color = true;
            index += 1;
        } else if first == "--plain" {
            plain = true;
            index += 1;
        } else {
            break;
        }
    }
    let home = codex_home(owned_home.as_deref());
    let ui = Ui::new(no_color, plain);
    let command = args.get(index).ok_or_else(|| "Missing command".to_string())?;
    index += 1;

    match command.as_str() {
        "save" => {
            let name = args.get(index).ok_or_else(|| "Save requires a profile name".to_string())?;
            index += 1;
            let mut owned_source: Option<String> = None;
            let mut force = false;
            while let Some(option) = args.get(index) {
                if option == "--force" {
                    force = true;
                } else if option == "--from" {
                    index += 1;
                    owned_source = Some(
                        args.get(index)
                            .ok_or_else(|| "--from requires a path".to_string())?
                            .clone(),
                    );
                } else if let Some(value) = option.strip_prefix("--from=") {
                    if value.is_empty() {
                        return Err("--from requires a path".to_string());
                    }
                    owned_source = Some(value.to_string());
                } else {
                    return Err(format!("Unknown save option: {option}"));
                }
                index += 1;
            }
            save(&home, name, owned_source.as_deref(), force, &ui)
        }
        "use" => {
            let name = args.get(index).ok_or_else(|| "Use requires a profile name".to_string())?;
            require_no_more_args(args, index + 1)?;
            use_profile(&home, name, &ui)
        }
        "list" => {
            // `list --plain` gives the stable `* name` script format.
            let mut list_ui = ui;
            if let Some(option) = args.get(index) {
                if option == "--plain" {
                    list_ui.plain = true;
                    index += 1;
                } else if option.starts_with('-') {
                    return Err(format!("Unknown list option: {option}"));
                }
            }
            require_no_more_args(args, index)?;
            list_profiles(&home, &list_ui)
        }
        "remove" | "delete" | "rm" => {
            let name = args.get(index).ok_or_else(|| "Remove requires a profile name".to_string())?;
            require_no_more_args(args, index + 1)?;
            remove_profile(&home, name, &ui)
        }
        "new" | "restart" | "clear" | "logout-local" => {
            require_no_more_args(args, index)?;
            clear_active(&home, &ui)
        }
        "completion" | "completions" | "complete" => {
            let shell = args.get(index).ok_or_else(|| {
                "Completion requires a shell: bash, zsh, fish, or powershell".to_string()
            })?;
            require_no_more_args(args, index + 1)?;
            print_completion(shell)
        }
        "path" => {
            require_no_more_args(args, index)?;
            show_paths(&home, &ui);
            Ok(())
        }
        "help" => {
            require_no_more_args(args, index)?;
            usage();
            Ok(())
        }
        other => Err(format!("Unknown command: {other}")),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage();
        process::exit(if args.is_empty() { 2 } else { 0 });
    }
    if let Err(error) = run(&args) {
        print_error(&error);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_auth, clear_active, display_width, expand_user, pad_to, print_completion,
        profile_auth, run, save, use_profile, validate_name, Ui,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_home() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "codex-accounts-test-{}-{nonce}-{}",
            process::id(),
            rand_suffix()
        ));
        fs::create_dir_all(&path).expect("create temporary home");
        path
    }

    fn rand_suffix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ (process::id() as u64))
            .unwrap_or(42)
    }

    #[test]
    fn profile_names_are_safe() {
        assert!(validate_name("personal").is_ok());
        assert!(validate_name("work-2026").is_ok());
        assert!(validate_name("a.b_c-d9").is_ok());
        assert!(validate_name("../escape").is_err());
        assert!(validate_name("_leading").is_err());
        assert!(validate_name("-leading").is_err());
        assert!(validate_name(".leading").is_err());
        assert!(validate_name("").is_err());
        assert!(validate_name(&"a".repeat(65)).is_err());
        assert!(validate_name(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn tilde_expands_to_home() {
        // `~` alone must not panic (previous out-of-bounds slice bug).
        let expanded = expand_user("~");
        assert!(!expanded.as_os_str().is_empty());
    }

    #[test]
    fn equals_forms_parse() {
        let home = temporary_home();
        let home_str = home.to_string_lossy().into_owned();
        fs::write(home.join("auth.json"), br#"{"account":"a"}"#).expect("write auth");
        let args = vec![
            format!("--codex-home={home_str}"),
            "save".to_string(),
            "personal".to_string(),
        ];
        run(&args).expect("save via --codex-home=");
        assert!(home.join("account-profiles/personal/auth.json").is_file());
        fs::remove_dir_all(home).expect("cleanup");
    }

    #[test]
    fn saves_and_activates_a_profile() {
        let home = temporary_home();
        let active = active_auth(&home);
        let ui = Ui::test();
        fs::write(&active, br#"{"account":"personal"}"#).expect("write active auth");

        save(&home, "personal", None, false, &ui).expect("save profile");
        assert_eq!(fs::read(profile_auth(&home, "personal").unwrap()).unwrap(), fs::read(&active).unwrap());

        fs::write(&active, br#"{"account":"work"}"#).expect("replace active auth");
        use_profile(&home, "personal", &ui).expect("activate profile");
        assert_eq!(fs::read(&active).unwrap(), br#"{"account":"personal"}"#);

        // `use` must not chmod the home's parent directory.
        let grandparent = home.parent().expect("home has parent");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(grandparent).expect("stat parent").permissions().mode() & 0o777;
            assert_ne!(mode, 0o700, "grandparent permissions must be untouched");
        }

        fs::remove_dir_all(home).expect("remove temporary home");
    }

    #[test]
    fn delete_aliases_remove_a_profile() {
        for alias in ["remove", "delete", "rm"] {
            let home = temporary_home();
            let home_str = home.to_string_lossy().into_owned();
            fs::write(home.join("auth.json"), br#"{"account":"a"}"#).expect("write auth");
            run(&[
                format!("--codex-home={home_str}"),
                "save".to_string(),
                "victim".to_string(),
            ])
            .expect("save");
            assert!(home.join("account-profiles/victim/auth.json").is_file());
            run(&[format!("--codex-home={home_str}"), alias.to_string(), "victim".to_string()])
                .expect("remove via alias");
            assert!(!home.join("account-profiles/victim/auth.json").exists());
            fs::remove_dir_all(home).expect("cleanup");
        }
    }

    #[test]
    fn new_aliases_clear_active_but_keep_profiles() {
        for alias in ["new", "restart", "clear", "logout-local"] {
            let home = temporary_home();
            let home_str = home.to_string_lossy().into_owned();
            let ui = Ui::test();
            fs::write(home.join("auth.json"), br#"{"account":"live"}"#).expect("write auth");
            save(&home, "kept", None, false, &ui).expect("save profile");
            run(&[format!("--codex-home={home_str}"), alias.to_string()]).expect("clear active");
            assert!(!home.join("auth.json").exists(), "active auth must be gone");
            assert!(
                home.join("account-profiles/kept/auth.json").is_file(),
                "saved profile must survive"
            );
            // Clearing twice is not an error (idempotent fresh-login flow).
            run(&[format!("--codex-home={home_str}"), alias.to_string()]).expect("clear again");
            fs::remove_dir_all(home).expect("cleanup");
        }
    }

    #[test]
    fn clear_active_function_is_idempotent() {
        let home = temporary_home();
        let ui = Ui::test();
        fs::write(home.join("auth.json"), br#"{"account":"x"}"#).expect("write auth");
        clear_active(&home, &ui).expect("first clear");
        clear_active(&home, &ui).expect("second clear is ok");
        fs::remove_dir_all(home).expect("cleanup");
    }

    #[test]
    fn completion_rejects_unknown_shell() {
        assert!(print_completion("bash").is_ok());
        assert!(print_completion("zsh").is_ok());
        assert!(print_completion("fish").is_ok());
        assert!(print_completion("powershell").is_ok());
        assert!(print_completion("tcsh").is_err());
        // Embedded scripts must stay in sync with completions/ and support
        // profile-name completion (typing `a<TAB>` completes `abcd`).
        assert!(include_str!("../completions/codex-accounts.bash").contains("_codex_accounts"));
        assert!(include_str!("../completions/codex-accounts.bash").contains("account-profiles"));
        assert!(include_str!("../completions/codex-accounts.zsh").contains("_codex-accounts"));
        assert!(include_str!("../completions/codex-accounts.fish").contains("__codex_accounts_profiles"));
        assert!(include_str!("../completions/codex-accounts.ps1").contains("Register-ArgumentCompleter"));
    }

    #[test]
    fn ui_measurement_is_exact() {
        assert_eq!(display_width("personal"), 8);
        assert_eq!(display_width("a.b_c-d9"), 8);
        assert_eq!(pad_to("ab", 4), "ab  ");
        assert_eq!(pad_to("abcd", 4), "abcd");
        // Longer than width is never truncated, only left-aligned.
        assert_eq!(pad_to("abcde", 4), "abcde");
        // Table column: every padded name has the same visible width.
        let names = ["a", "abcd", "work-2026"];
        let width = names
            .iter()
            .map(|name| display_width(name))
            .max()
            .unwrap();
        assert_eq!(width, 9);
        for name in names {
            assert_eq!(display_width(&pad_to(name, width)), width);
        }
    }

    #[test]
    fn ui_paint_respects_color_flag() {
        let colorful = Ui {
            color: true,
            plain: false,
        };
        let plain_ui = Ui::test();
        assert!(colorful.green("✓").contains("\x1b[32m"));
        assert!(colorful.bold("x").contains("\x1b[1m"));
        assert_eq!(plain_ui.green("✓"), "✓");
        assert_eq!(plain_ui.tick(), "✓");
        // `Ui::new(no_color=true)` never enables color, even on a tty.
        assert!(!Ui::new(true, false).color);
    }

    #[test]
    fn global_ui_flags_parse() {
        let home = temporary_home();
        let home_str = home.to_string_lossy().into_owned();
        fs::write(home.join("auth.json"), br#"{"account":"a"}"#).expect("write auth");
        for extra in [["--plain"].as_slice(), ["--no-color"].as_slice(), ["--no-color", "--plain"].as_slice()] {
            let mut args = vec![format!("--codex-home={home_str}")];
            args.extend(extra.iter().map(|flag| flag.to_string()));
            args.push("list".to_string());
            run(&args).expect("list with UI flags");
        }
        // `list --plain` keeps the stable script format.
        run(&[format!("--codex-home={home_str}"), "list".to_string(), "--plain".to_string()])
            .expect("list --plain");
        fs::remove_dir_all(home).expect("cleanup");
    }
}
