use serde_json::Value;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_DIR: &str = "account-profiles";

type Result<T> = std::result::Result<T, String>;

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

fn save(home: &Path, name: &str, source: Option<&str>, force: bool) -> Result<()> {
    let source = source.map(expand_user).unwrap_or_else(|| active_auth(home));
    let destination = profile_auth(home, name)?;
    atomic_copy(&source, &destination, force)?;
    println!("Saved profile '{name}'.");
    Ok(())
}

fn use_profile(home: &Path, name: &str) -> Result<()> {
    let source = profile_auth(home, name)?;
    let destination = active_auth(home);
    atomic_copy(&source, &destination, true)?;
    println!("Activated profile '{name}'. Start a new Codex process to use it.");
    Ok(())
}

fn list_profiles(home: &Path) -> Result<()> {
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
            println!("No saved profiles.");
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
        println!("No saved profiles.");
        return Ok(());
    }
    for name in names {
        let profile = fs::read(profile_auth(home, &name)?).ok();
        let marker = if active_bytes.is_some() && profile == active_bytes { '*' } else { ' ' };
        println!("{marker} {name}");
    }
    Ok(())
}

fn remove_profile(home: &Path, name: &str) -> Result<()> {
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
    println!("Removed profile '{name}'.");
    Ok(())
}

fn show_paths(home: &Path) {
    println!("CODEX_HOME: {}", home.display());
    println!("Active auth: {}", active_auth(home).display());
    println!("Profiles:    {}", profile_root(home).display());
}

fn usage() {
    eprintln!(
        "Usage: codex-accounts [--codex-home PATH] <save|use|list|remove|path> ...\n\n\
         save NAME [--from AUTH_JSON] [--force]\n\
         use NAME\n\
         list\n\
         remove NAME\n\
         path"
    );
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
    // Accept both `--codex-home PATH` and `--codex-home=PATH`
    // (argparse supports both, so the Rust port must too).
    if let Some(first) = args.get(index) {
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
        }
    }
    let home = codex_home(owned_home.as_deref());
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
            save(&home, name, owned_source.as_deref(), force)
        }
        "use" => {
            let name = args.get(index).ok_or_else(|| "Use requires a profile name".to_string())?;
            require_no_more_args(args, index + 1)?;
            use_profile(&home, name)
        }
        "list" => {
            require_no_more_args(args, index)?;
            list_profiles(&home)
        }
        "remove" => {
            let name = args.get(index).ok_or_else(|| "Remove requires a profile name".to_string())?;
            require_no_more_args(args, index + 1)?;
            remove_profile(&home, name)
        }
        "path" => {
            require_no_more_args(args, index)?;
            show_paths(&home);
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
        eprintln!("Error: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{active_auth, expand_user, profile_auth, run, save, use_profile, validate_name};
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
        fs::write(&active, br#"{"account":"personal"}"#).expect("write active auth");

        save(&home, "personal", None, false).expect("save profile");
        assert_eq!(fs::read(profile_auth(&home, "personal").unwrap()).unwrap(), fs::read(&active).unwrap());

        fs::write(&active, br#"{"account":"work"}"#).expect("replace active auth");
        use_profile(&home, "personal").expect("activate profile");
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
}
