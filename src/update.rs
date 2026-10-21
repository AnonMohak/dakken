//! Updates. `dakken --check-update` asks GitHub for the latest release. The
//! app also asks once a day in the background, and the menu shows a newer
//! version; `u` updates dakken with the tool that installed it. It uses
//! `curl`, so dakken needs no HTTP library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

const LATEST: &str = "https://api.github.com/repos/AnonMohak/dakken/releases/latest";
const DOWNLOAD: &str = "https://github.com/AnonMohak/dakken/releases/download";
const RELEASES: &str = "https://github.com/AnonMohak/dakken/releases/latest";
const GIT: &str = "https://github.com/AnonMohak/dakken.git";
/// The app asks GitHub at most once in this many seconds.
const CHECK_EVERY: u64 = 86_400;

/// A version such as "v0.2.0" or "0.2.0" as numbers.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    Some((next()?, next()?, next()?))
}

/// True if the release `latest` is newer than the version `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    matches!((parse_version(current), parse_version(latest)), (Some(c), Some(l)) if l > c)
}

/// The "tag_name" value of the GitHub release JSON.
fn tag_name(json: &str) -> Option<&str> {
    let rest = &json[json.find("\"tag_name\"")? + "\"tag_name\"".len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    rest.split('"').next()
}

/// The message for this version and the latest release tag.
fn message(current: &str, latest: &str) -> String {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) if l > c => format!(
            "A new version is available: {latest} (you have v{current}).\n\
             Get it from {RELEASES}\n"
        ),
        (Some(_), Some(_)) => format!("dakken v{current} is the latest version.\n"),
        _ => format!("error: cannot read the version \"{latest}\"\n"),
    }
}

/// The tag of the latest release, from GitHub, or the error to print.
fn latest_tag() -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-sSfL",
            "--max-time",
            "10",
            "-H",
            "User-Agent: dakken",
            LATEST,
        ])
        .output();
    let json = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            return Err(format!("error: cannot reach GitHub: {}\n", err.trim()));
        }
        Err(e) => return Err(format!("error: cannot run curl: {e}\n")),
    };
    tag_name(&json)
        .map(String::from)
        .ok_or_else(|| "error: the GitHub answer has no release tag\n".to_string())
}

/// Ask GitHub, and give the text to print.
pub fn check() -> String {
    match latest_tag() {
        Ok(tag) => message(env!("CARGO_PKG_VERSION"), &tag),
        Err(e) => e,
    }
}

/// The file that keeps the time of the last check and the tag it found.
pub fn cache_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("update-check.tsv"))
}

/// "time\ttag" from the cache file.
fn parse_cache(text: &str) -> Option<(u64, String)> {
    let (time, tag) = text.trim().split_once('\t')?;
    Some((time.parse().ok()?, tag.to_string()))
}

/// The tag of a release newer than this version, if there is one. A check
/// from the last day comes from the cache file; else GitHub is asked and
/// the answer is saved. An error gives `None`, so a check never gets in
/// the way.
pub fn newer_release(
    cache: Option<&Path>,
    now: u64,
    fetch: impl FnOnce() -> Option<String>,
) -> Option<String> {
    let cached = cache
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|text| parse_cache(&text));
    let tag = match cached {
        Some((time, tag)) if now.saturating_sub(time) < CHECK_EVERY => tag,
        _ => {
            let tag = fetch()?;
            if let Some(path) = cache {
                if let Some(dir) = path.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::write(path, format!("{now}\t{tag}\n"));
            }
            tag
        }
    };
    is_newer(env!("CARGO_PKG_VERSION"), &tag).then_some(tag)
}

/// Check in a background thread. The receiver gets the newer tag, if any.
pub fn check_in_background() -> mpsc::Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let cache = cache_path();
        if let Some(tag) = newer_release(cache.as_deref(), now, || latest_tag().ok()) {
            let _ = send.send(tag);
        }
    });
    receive
}

/// How dakken was installed, from the folder of the running program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    Scoop,
    Winget,
    Homebrew,
    Cargo,
    /// A file from the Releases page, in any folder.
    File,
}

pub fn install_kind(exe: &Path) -> Install {
    let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    if path.contains("/scoop/apps/dakken/") || path.contains("/scoop/shims/") {
        Install::Scoop
    } else if path.contains("/winget/packages/") || path.contains("/winget/links/") {
        Install::Winget
    } else if path.contains("/cellar/dakken/")
        || path.contains("/homebrew/")
        || path.contains("linuxbrew/")
    {
        Install::Homebrew
    } else if path.contains("/.cargo/bin/") {
        Install::Cargo
    } else {
        Install::File
    }
}

impl Install {
    /// The command that updates dakken, for the install tools.
    pub fn command(self) -> Option<&'static [&'static str]> {
        match self {
            Install::Scoop => Some(&["scoop", "update", "dakken"]),
            Install::Winget => Some(&["winget", "upgrade", "--id", "AnonMohak.dakken"]),
            Install::Homebrew => Some(&["brew", "upgrade", "dakken"]),
            Install::Cargo => Some(&["cargo", "install", "--git", GIT, "--force"]),
            Install::File => None,
        }
    }
}

/// Update dakken after the app has closed, and give the text to print.
/// `tag` is the newer release.
pub fn update(tag: &str) -> String {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => return format!("error: cannot find dakken: {e}\nGet {tag} from {RELEASES}\n"),
    };
    let install = install_kind(&exe);
    let Some(args) = install.command() else {
        println!("Downloading dakken {tag}...");
        return match self_update(&exe, tag) {
            Ok(()) => format!("dakken is updated to {tag}. Start it again.\n"),
            Err(e) => format!("error: {e}\nGet dakken {tag} from {RELEASES}\n"),
        };
    };
    let line = args.join(" ");
    if cfg!(windows) {
        // A running .exe cannot be replaced on Windows, so a new window
        // waits until dakken has closed, then updates it.
        return match update_after_exit(&line) {
            Ok(()) => format!(
                "The update to {tag} runs in a new window: {line}\n\
                 Start dakken again when it is done.\n"
            ),
            Err(e) => format!("error: cannot start the update: {e}\nRun it yourself: {line}\n"),
        };
    }
    println!("Updating to {tag}: {line}");
    match Command::new(args[0]).args(&args[1..]).status() {
        Ok(status) if status.success() => format!("dakken is updated to {tag}.\n"),
        Ok(_) => format!("error: the update did not finish. Run it yourself: {line}\n"),
        Err(e) => format!(
            "error: cannot run {}: {e}\nRun it yourself: {line}\n",
            args[0]
        ),
    }
}

/// Windows: a new PowerShell window waits for this process to end, then
/// runs `line` and waits for Enter.
#[cfg(windows)]
fn update_after_exit(line: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    let script = format!(
        "Wait-Process -Id {} -ErrorAction SilentlyContinue; {line}; \
         Read-Host 'Press Enter to close'",
        std::process::id()
    );
    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map(|_| ())
}

#[cfg(not(windows))]
fn update_after_exit(_line: &str) -> std::io::Result<()> {
    Ok(())
}

/// The release file for this system, for example
/// `dakken-v0.5.1-x86_64-pc-windows-msvc.exe`.
fn asset_name(tag: &str) -> String {
    let exe = if cfg!(windows) { ".exe" } else { "" };
    format!("dakken-{tag}-{}{exe}", env!("DAKKEN_TARGET"))
}

/// The hash of `name` in a `SHA256SUMS` file (`<hash>  <name>` lines; a
/// `*` before the name marks binary mode).
fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim().trim_start_matches('*');
        (file == name).then(|| hash.to_lowercase())
    })
}

/// Download `url` with curl into `path`, or into memory with no path.
fn download(url: &str, path: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut curl = Command::new("curl");
    curl.args([
        "-sSfL",
        "--max-time",
        "300",
        "-H",
        "User-Agent: dakken",
        url,
    ]);
    if let Some(path) = path {
        curl.arg("-o").arg(path);
    }
    match curl.output() {
        Ok(o) if o.status.success() => Ok(o.stdout),
        Ok(o) => Err(format!(
            "cannot download {url}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot run curl: {e}")),
    }
}

/// Replace the program at `exe` with the release `tag` for this system.
/// The download must match the release's `SHA256SUMS`. The running program
/// is renamed to `dakken.old` first, because Windows cannot overwrite it;
/// the next start deletes it.
pub fn self_update(exe: &Path, tag: &str) -> Result<(), String> {
    let name = asset_name(tag);
    let sums = download(&format!("{DOWNLOAD}/{tag}/SHA256SUMS"), None)?;
    let sums = String::from_utf8_lossy(&sums);
    let want = expected_hash(&sums, &name).ok_or(format!("no checksum for {name}"))?;
    let new = exe.with_file_name(if cfg!(windows) {
        "dakken.new.exe"
    } else {
        "dakken.new"
    });
    download(&format!("{DOWNLOAD}/{tag}/{name}"), Some(&new))?;
    let got = fs::read(&new).map(|bytes| crate::sha256::hex(&bytes));
    if got.as_deref().ok() != Some(want.as_str()) {
        let _ = fs::remove_file(&new);
        return Err(format!(
            "the download of {name} does not match its checksum"
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let old = old_path(exe);
    let _ = fs::remove_file(&old);
    let cannot = |e: std::io::Error| format!("cannot replace {}: {e}", exe.display());
    fs::rename(exe, &old).map_err(|e| {
        let _ = fs::remove_file(&new);
        cannot(e)
    })?;
    if let Err(e) = fs::rename(&new, exe) {
        // Put the old program back.
        let _ = fs::rename(&old, exe);
        let _ = fs::remove_file(&new);
        return Err(cannot(e));
    }
    if !cfg!(windows) {
        let _ = fs::remove_file(&old);
    }
    Ok(())
}

/// Delete `dakken.old` (`dakken.old.exe` on Windows) next to the program,
/// if a self-update left it.
pub fn remove_old_exe() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = fs::remove_file(old_path(&exe));
    }
}

/// The name the running program gets during a self-update.
fn old_path(exe: &Path) -> PathBuf {
    let old = if cfg!(windows) {
        "dakken.old.exe"
    } else {
        "dakken.old"
    };
    exe.with_file_name(old)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(parse_version("v0.10.2"), Some((0, 10, 2)));
        assert!(parse_version("v0.10.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("main"), None);
        assert!(is_newer("0.5.0", "v0.5.1") && !is_newer("0.5.0", "v0.5.0"));
        assert!(!is_newer("0.5.0", "nightly"));
    }

    #[test]
    fn tag_name_is_read_from_the_json() {
        let json = r#"{"url": "x", "tag_name" : "v0.3.0", "name": "v0.3.0"}"#;
        assert_eq!(tag_name(json), Some("v0.3.0"));
        assert_eq!(tag_name("{}"), None);
    }

    #[test]
    fn message_says_if_there_is_a_newer_version() {
        assert!(message("0.2.0", "v0.3.0").starts_with("A new version is available: v0.3.0"));
        assert_eq!(
            message("0.2.0", "v0.2.0"),
            "dakken v0.2.0 is the latest version.\n"
        );
        assert!(message("0.3.0", "v0.2.0").contains("is the latest version"));
        assert!(message("0.2.0", "nightly").starts_with("error"));
    }

    #[test]
    fn a_check_from_the_last_day_comes_from_the_cache() {
        let dir = std::env::temp_dir().join(format!("dakken-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("update-check.tsv");
        let never = || -> Option<String> { panic!("GitHub is asked") };
        // No cache: ask, and save the answer.
        let found = newer_release(Some(&cache), 1_000, || Some("v99.0.0".into()));
        assert_eq!(found.as_deref(), Some("v99.0.0"));
        assert_eq!(fs::read_to_string(&cache).unwrap(), "1000\tv99.0.0\n");
        // Less than a day later: the cache answers.
        assert_eq!(
            newer_release(Some(&cache), 1_000 + CHECK_EVERY - 1, never).as_deref(),
            Some("v99.0.0")
        );
        // A day later: ask again. Not newer, so no update.
        let old = newer_release(Some(&cache), 1_000 + CHECK_EVERY, || Some("v0.0.1".into()));
        assert_eq!(old, None);
        // No answer from GitHub: nothing.
        assert_eq!(newer_release(None, 0, || None), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_checksum_is_found_by_file_name() {
        let sums = "AB12  dakken-v0.6.0-x86_64-pc-windows-msvc.exe\n\
                    cd34 *dakken-v0.6.0-x86_64-unknown-linux-musl\n";
        assert_eq!(
            expected_hash(sums, "dakken-v0.6.0-x86_64-pc-windows-msvc.exe").as_deref(),
            Some("ab12")
        );
        assert_eq!(
            expected_hash(sums, "dakken-v0.6.0-x86_64-unknown-linux-musl").as_deref(),
            Some("cd34")
        );
        assert_eq!(
            expected_hash(sums, "dakken-v0.6.0-aarch64-apple-darwin"),
            None
        );
        let name = asset_name("v0.6.0");
        assert!(name.starts_with("dakken-v0.6.0-") && name.contains(env!("DAKKEN_TARGET")));
    }

    /// Downloads a real release into a temporary folder. Run it with
    /// `cargo test -- --ignored self_update`.
    #[test]
    #[ignore]
    fn self_update_replaces_a_copy_with_a_real_release() {
        let dir = std::env::temp_dir().join(format!("dakken-self-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(if cfg!(windows) {
            "dakken.exe"
        } else {
            "dakken"
        });
        fs::write(&exe, b"old program").unwrap();
        self_update(&exe, "v0.5.0").unwrap();
        let out = Command::new(&exe).arg("--version").output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "dakken 0.5.0");
        assert_eq!(
            fs::read(old_path(&exe)).ok().as_deref(),
            cfg!(windows).then_some(&b"old program"[..])
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_install_tool_comes_from_the_folder() {
        let kind = |p: &str| install_kind(Path::new(p));
        assert_eq!(
            kind(r"C:\Users\a\scoop\apps\dakken\current\dakken.exe"),
            Install::Scoop
        );
        assert_eq!(
            kind(
                r"C:\Users\a\AppData\Local\Microsoft\WinGet\Packages\AnonMohak.dakken_x\dakken.exe"
            ),
            Install::Winget
        );
        assert_eq!(
            kind("/opt/homebrew/Cellar/dakken/0.5.0/bin/dakken"),
            Install::Homebrew
        );
        assert_eq!(kind("/home/a/.linuxbrew/bin/dakken"), Install::Homebrew);
        assert_eq!(kind(r"C:\Users\a\.cargo\bin\dakken.exe"), Install::Cargo);
        assert_eq!(kind(r"C:\Tools\dakken.exe"), Install::File);
        assert_eq!(Install::File.command(), None);
        assert_eq!(
            Install::Scoop.command().unwrap().join(" "),
            "scoop update dakken"
        );
    }
}
