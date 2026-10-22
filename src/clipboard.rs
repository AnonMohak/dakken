//! The system clipboard, through the clipboard command of each system. The
//! OSC 52 escape code (see `card.rs`) does not work in every terminal:
//! GNOME Terminal, macOS Terminal.app and tmux ignore it. So the score card
//! goes through both.

use std::process::{Command, Stdio};

/// Put `text` on the system clipboard. Gives false when no clipboard
/// command worked, for example over SSH or with no clipboard tool installed.
#[cfg(windows)]
pub fn copy(text: &str) -> bool {
    use std::os::windows::process::CommandExt;
    // With no console, PowerShell cannot change the console modes of dakken.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // An environment variable is UTF-16 on Windows, so emoji stay whole.
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Set-Clipboard -Value $env:DAKKEN_CLIPBOARD",
        ])
        .env("DAKKEN_CLIPBOARD", text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(not(windows))]
pub fn copy(text: &str) -> bool {
    let set = |name| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    let mut tools: Vec<&[&str]> = Vec::new();
    if cfg!(target_os = "macos") {
        tools.push(&["pbcopy"]);
    }
    if set("WAYLAND_DISPLAY") {
        tools.push(&["wl-copy"]);
    }
    if set("DISPLAY") {
        tools.push(&["xclip", "-selection", "clipboard"]);
        tools.push(&["xsel", "--clipboard", "--input"]);
    }
    tools.into_iter().any(|tool| pipe(tool, text))
}

/// Run `tool` with `text` on its standard input.
#[cfg(not(windows))]
fn pipe(tool: &[&str], text: &str) -> bool {
    use std::io::Write;
    let child = Command::new(tool[0])
        .args(&tool[1..])
        // pbcopy reads the input in the encoding of the locale.
        .env("LC_CTYPE", "UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return false;
    };
    let written = child
        .stdin
        .take()
        .is_some_and(|mut input| input.write_all(text.as_bytes()).is_ok());
    child.wait().is_ok_and(|s| s.success()) && written
}
