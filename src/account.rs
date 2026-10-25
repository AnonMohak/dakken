//! The player's profile on the dakken leaderboard (me-momo.co.in/dakken).
//! The server knows a player by a name; each device has its own secret key.
//! This computer keeps its name and key in `account.tsv` in the data folder,
//! one line: the name, a tab, then the key.
//!
//! `dakken name`, `dakken link` and `dakken recover` make, join or get back a
//! profile; then the app sends its runs (`upload`).

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

/// The API of the leaderboard. `DAKKEN_API` changes it, for a local server.
const API: &str = "https://me-momo.co.in/dakken/api";
/// A call that takes longer stops; the app never waits more than this.
const TIMEOUT: Duration = Duration::from_secs(10);

fn api() -> String {
    std::env::var("DAKKEN_API")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| API.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    pub key: String,
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("account.tsv"))
}

/// The account of this computer, if it has one.
pub fn load() -> Option<Account> {
    parse(&fs::read_to_string(default_path()?).ok()?)
}

fn parse(text: &str) -> Option<Account> {
    let (name, key) = text.lines().next()?.split_once('\t')?;
    (!name.is_empty() && !key.is_empty()).then(|| Account {
        name: name.to_string(),
        key: key.to_string(),
    })
}

/// Keep the account. On Unix, only this user can read the file.
pub fn save(account: &Account) -> io::Result<()> {
    let path = default_path().ok_or_else(|| io::Error::other("no data folder"))?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, format!("{}\t{}\n", account.name, account.key))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// The label of this device in the device list, like "app on windows".
fn label() -> String {
    format!("app on {}", std::env::consts::OS)
}

enum Method {
    Get,
    Post,
    Put,
}

/// One call to the API: the status and the JSON answer. An error is a
/// short text for the player.
fn call(
    method: Method,
    path: &str,
    key: Option<&str>,
    body: Value,
) -> Result<(u16, Value), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let url = format!("{}{path}", api());
    let auth = key.map(|k| format!("Bearer {k}"));
    let result = match method {
        Method::Get => {
            let mut request = agent.get(&url);
            if let Some(auth) = &auth {
                request = request.header("Authorization", auth);
            }
            request.call()
        }
        Method::Post | Method::Put => {
            let mut request = match method {
                Method::Post => agent.post(&url),
                _ => agent.put(&url),
            };
            if let Some(auth) = &auth {
                request = request.header("Authorization", auth);
            }
            request
                .header("Content-Type", "application/json")
                .send(body.to_string())
        }
    };
    let mut response = result.map_err(|e| format!("no answer from the leaderboard ({e})"))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// The reason in an error answer, in words for the player.
fn reason(status: u16, body: &Value) -> String {
    let error = body["error"].as_str().unwrap_or_default();
    match error {
        "bad_name" => body["reason"]
            .as_str()
            .unwrap_or("not a good name")
            .to_string(),
        "name_taken" => "that name is taken".to_string(),
        "rename_too_soon" => "you changed your name a short time ago; try again later".to_string(),
        "bad_code" => "that code does not work (wrong, used, or older than 10 minutes)".to_string(),
        "bad_key" => {
            "this computer's key does not work any more; use `dakken link` or `dakken recover`"
                .to_string()
        }
        "too_many" => "too many tries; try again in an hour".to_string(),
        "" => format!("the leaderboard answered {status}"),
        other => other.replace('_', " "),
    }
}

fn text(body: &Value, field: &str) -> String {
    body[field].as_str().unwrap_or_default().to_string()
}

/// Make a profile with this computer as its first device. Gives the account
/// and the recovery code (shown one time).
pub fn create(name: &str) -> Result<(Account, String), String> {
    let (status, body) = call(
        Method::Post,
        "/players",
        None,
        json!({ "name": name, "device": label() }),
    )?;
    if status != 201 {
        return Err(reason(status, &body));
    }
    let account = Account {
        name: text(&body, "name"),
        key: text(&body, "device_key"),
    };
    Ok((account, text(&body, "recovery_code")))
}

/// Change the name. Gives the new name.
pub fn rename(account: &Account, name: &str) -> Result<String, String> {
    let (status, body) = call(
        Method::Put,
        "/me/name",
        Some(&account.key),
        json!({ "name": name }),
    )?;
    if status != 200 {
        return Err(reason(status, &body));
    }
    Ok(text(&body, "name"))
}

/// A link code for one more device, and the Unix time when it stops.
pub fn link_code(account: &Account) -> Result<(String, u64), String> {
    let (status, body) = call(Method::Post, "/me/link-code", Some(&account.key), json!({}))?;
    if status != 201 {
        return Err(reason(status, &body));
    }
    Ok((
        text(&body, "code"),
        body["expires_at"].as_u64().unwrap_or_default(),
    ))
}

/// Join a profile with a link code from another device.
pub fn join(code: &str) -> Result<Account, String> {
    let (status, body) = call(
        Method::Post,
        "/link",
        None,
        json!({ "code": code, "device": label() }),
    )?;
    if status != 201 {
        return Err(reason(status, &body));
    }
    Ok(Account {
        name: text(&body, "name"),
        key: text(&body, "device_key"),
    })
}

/// Get a profile back with its recovery code. Gives the account and a new
/// recovery code; the old one stops.
pub fn recover(code: &str) -> Result<(Account, String), String> {
    let body = json!({ "recovery_code": code, "device": label() });
    let (status, body) = call(Method::Post, "/recover", None, body)?;
    if status != 201 {
        return Err(reason(status, &body));
    }
    let account = Account {
        name: text(&body, "name"),
        key: text(&body, "device_key"),
    };
    Ok((account, text(&body, "recovery_code")))
}

/// The devices of the profile: (label, this device).
pub fn devices(account: &Account) -> Result<Vec<(String, bool)>, String> {
    let (status, body) = call(Method::Get, "/me/devices", Some(&account.key), Value::Null)?;
    if status != 200 {
        return Err(reason(status, &body));
    }
    Ok((body["devices"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter())
    .map(|d| (text(d, "label"), d["this"].as_bool().unwrap_or(false)))
    .collect())
}

/// Why a run did not go: no connection (try again later), or the server
/// said no (the run goes away).
pub enum SendError {
    Offline,
    Refused(String),
}

/// Send a run (the JSON of `POST /api/runs`). Gives the server's answer:
/// whether it is on a board, its WPM, and whether it is a new best.
pub fn send_run(account: &Account, run: &Value) -> Result<Value, SendError> {
    let (status, body) = call(Method::Post, "/runs", Some(&account.key), run.clone())
        .map_err(|_| SendError::Offline)?;
    match status {
        201 => Ok(body),
        // A server that is down is like no connection.
        500.. => Err(SendError::Offline),
        _ => Err(SendError::Refused(reason(status, &body))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_account_file_is_one_line_with_a_tab() {
        let account = parse("momo\tabc123\n").unwrap();
        assert_eq!(account.name, "momo");
        assert_eq!(account.key, "abc123");
        assert_eq!(parse(""), None);
        assert_eq!(parse("momo"), None);
        assert_eq!(parse("\tkey"), None);
    }

    #[test]
    fn errors_are_words_for_the_player() {
        let err = |e: &str| {
            reason(
                400,
                &json!({ "error": e, "reason": "at least 4 characters" }),
            )
        };
        assert_eq!(err("bad_name"), "at least 4 characters");
        assert_eq!(err("name_taken"), "that name is taken");
        assert_eq!(err("something_new"), "something new");
        assert_eq!(reason(500, &Value::Null), "the leaderboard answered 500");
    }
}
