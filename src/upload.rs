//! Runs to the leaderboard. With a profile on this computer (`dakken name`
//! or `dakken link`), each finished test goes to the server in a thread, so
//! the app never waits for the network.
//!
//! A time 30 or time 60 test goes with its seed and key log: the server makes
//! the same words again, replays the keys and counts the WPM itself, then it
//! goes on a speed board. Other tests go as a summary and count only for the
//! time typed this week.
//!
//! A run that finds no connection waits in `pending.tsv` (one JSON run on each
//! line) and goes with the next run. The server takes a run only in the first
//! hour after it ends, so older lines go.
//!
//! The badges and the XP go too, at the start and after each test, so the
//! website shows them. Each send has all of them, so a send that fails needs
//! no queue: the next one puts it right.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};

use crate::account::{self, Account, SendError};
use crate::engine::{Key, stats::TestResult};

/// The server takes a run only this long after it ends.
const MAX_AGE: u64 = 3600;

/// The key log in the server's format: [[ms since the first key, key], ...],
/// where a key is one character, "space", "bs" (Backspace) or "dw"
/// (Ctrl+Backspace). The times round down, so a key just before the end
/// stays before it.
pub fn key_log(log: &[(Duration, Key)]) -> Value {
    Value::Array(
        log.iter()
            .map(|(at, key)| {
                let key = match key {
                    Key::Char(c) => c.to_string(),
                    Key::Space => "space".to_string(),
                    Key::Backspace => "bs".to_string(),
                    Key::DeleteWord => "dw".to_string(),
                };
                json!([at.as_millis() as u64, key])
            })
            .collect(),
    )
}

/// The JSON of `POST /api/runs` for a finished test. `seed` is set for a
/// competitive test (time 30 or time 60); its key log goes too.
pub fn run_json(
    label: &str,
    result: &TestResult,
    seed: Option<u64>,
    log: &[(Duration, Key)],
    at: u64,
) -> Value {
    let mut run = json!({
        "mode": label,
        "at": at,
        "source": "app",
        "seconds": result.duration.as_secs_f64().clamp(0.5, 3600.0),
        "wpm": result.wpm.min(300.0),
        "acc": result.accuracy,
    });
    if let Some(seed) = seed {
        run["keyboard"] = json!("physical");
        run["seed"] = json!(seed.to_string());
        run["log"] = key_log(log);
    }
    run
}

/// The JSON of `PUT /api/me/progress`: the XP of this computer's history and
/// every badge with the time you got it.
pub fn progress_json(xp: u64, badges: &[(String, u64)]) -> Value {
    json!({
        "xp": xp,
        "badges": badges
            .iter()
            .map(|(name, at)| json!({ "name": name, "at": at }))
            .collect::<Vec<_>>(),
    })
}

/// Send the badges and the XP in a thread; the answer does not matter.
pub fn send_progress(account: Account, progress: Value) {
    std::thread::spawn(move || {
        let _ = account::send_progress(&account, &progress);
    });
}

fn pending_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("pending.tsv"))
}

/// The waiting runs that the server still takes.
fn fresh(text: &str, now: u64) -> Vec<Value> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|run| run["at"].as_u64().is_some_and(|at| at + MAX_AGE > now + 60))
        .collect()
}

fn write_pending(runs: &[Value]) {
    let Some(path) = pending_path() else { return };
    if runs.is_empty() {
        let _ = fs::remove_file(path);
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(mut file) = fs::File::create(path) {
        for run in runs {
            let _ = writeln!(file, "{run}");
        }
    }
}

/// The line for the results screen after the server's answer.
fn note(run: &Value, answer: &Value) -> Option<String> {
    if !answer["ranked"].as_bool().unwrap_or(false) {
        return None;
    }
    let wpm = answer["wpm"].as_f64().unwrap_or_default();
    let board = run["mode"]
        .as_str()
        .unwrap_or_default()
        .split(" · ")
        .next()
        .unwrap_or_default();
    Some(if answer["best"].as_bool().unwrap_or(false) {
        format!("leaderboard: new best ✓ {wpm:.0} wpm on {board}")
    } else {
        format!("leaderboard: sent ✓ {wpm:.0} wpm, not better than your best on {board}")
    })
}

/// Send the waiting runs, then `run` (if any), in a thread. The receiver
/// gets one line for the results screen, or nothing.
pub fn send(account: Account, run: Option<Value>, now: u64) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut queue = pending_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map_or_else(Vec::new, |text| fresh(&text, now));
        let mine = queue.len();
        queue.extend(run);
        let mut left = Vec::new();
        let mut line = None;
        for (i, item) in queue.into_iter().enumerate() {
            if !left.is_empty() {
                left.push(item);
                continue;
            }
            match account::send_run(&account, &item) {
                Ok(answer) if i == mine => line = note(&item, &answer),
                Ok(_) => {}
                // No connection: this run and the rest wait for the next time.
                Err(SendError::Offline) => {
                    if i == mine {
                        line =
                            Some("leaderboard: no connection; the run goes next time".to_string());
                    }
                    left.push(item);
                }
                // The server said no: the run goes away.
                Err(SendError::Refused(why)) if i == mine => {
                    line = Some(format!("leaderboard: not sent, {why}"))
                }
                Err(SendError::Refused(_)) => {}
            }
        }
        write_pending(&left);
        if let Some(line) = line {
            let _ = tx.send(line);
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(wpm: f64) -> TestResult {
        TestResult {
            wpm,
            raw_wpm: wpm,
            accuracy: 97.5,
            consistency: 80.0,
            chars: Default::default(),
            duration: Duration::from_secs(30),
            raw_per_second: Vec::new(),
            wpm_over_time: Vec::new(),
            slowest_word: None,
            fastest_burst: None,
        }
    }

    #[test]
    fn the_key_log_has_the_server_format() {
        let log = [
            (Duration::ZERO, Key::Char('a')),
            (Duration::from_micros(120_900), Key::Space),
            (Duration::from_millis(200), Key::Backspace),
            (Duration::from_millis(29_999), Key::DeleteWord),
        ];
        assert_eq!(
            key_log(&log),
            json!([[0, "a"], [120, "space"], [200, "bs"], [29_999, "dw"]])
        );
    }

    #[test]
    fn a_competitive_run_has_its_seed_and_key_log() {
        let log = [(Duration::ZERO, Key::Char('s'))];
        let run = run_json(
            "time 30 · dvorak",
            &result(80.0),
            Some(u64::MAX),
            &log,
            1000,
        );
        assert_eq!(run["mode"], "time 30 · dvorak");
        assert_eq!(run["seed"], "18446744073709551615");
        assert_eq!(run["keyboard"], "physical");
        assert_eq!(run["log"], json!([[0, "s"]]));
        assert_eq!(run["at"], 1000);
    }

    #[test]
    fn other_runs_are_a_summary() {
        let run = run_json("words 25", &result(999.0), None, &[], 5);
        assert_eq!(run["wpm"], 300.0);
        assert_eq!(run["seconds"], 30.0);
        assert!(run.get("log").is_none() && run.get("seed").is_none());
    }

    #[test]
    fn progress_has_the_xp_and_every_badge_with_its_time() {
        let badges = [
            ("first press".to_string(), 100),
            ("night owl".to_string(), 200),
        ];
        assert_eq!(
            progress_json(450, &badges),
            json!({
                "xp": 450,
                "badges": [{ "name": "first press", "at": 100 }, { "name": "night owl", "at": 200 }],
            })
        );
        assert_eq!(progress_json(0, &[]), json!({ "xp": 0, "badges": [] }));
    }

    #[test]
    fn waiting_runs_older_than_an_hour_go() {
        let text = format!(
            "{}\n{}\nnot json\n",
            json!({ "at": 10_000 }),
            json!({ "at": 1_000 })
        );
        let left = fresh(&text, 10_000 + 600);
        assert_eq!(left, vec![json!({ "at": 10_000 })]);
    }

    /// Types a time 30 run with the app's engine and prints its JSON, to
    /// check it against the server's replay: `cargo test print_a_typed_run
    /// -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_a_typed_run() {
        use crate::engine::{Mode, Session, words::seeded_words};
        use std::time::Instant;
        let mut next = seeded_words(777);
        let words: Vec<String> = (0..40).map(|_| next()).collect();
        let mut s = Session::new(Mode::Time(Duration::from_secs(30)), seeded_words(777));
        let start = Instant::now();
        let mut ms = 0u64;
        let mut key = |s: &mut Session, k: Key| {
            ms += 90 + (ms * 7 % 41);
            s.handle(k, start + Duration::from_millis(ms));
        };
        for (i, word) in words.iter().enumerate() {
            if i == 3 {
                key(&mut s, Key::Char('x'));
                key(&mut s, Key::Backspace);
            }
            for c in word.chars() {
                key(&mut s, Key::Char(c));
            }
            key(&mut s, Key::Space);
        }
        s.tick(start + Duration::from_secs(31));
        let r = s.result();
        println!("APP {:.4} {:.4}", r.wpm, r.accuracy);
        println!("RUN {}", run_json("time 30", &r, Some(777), s.log(), 0));
    }

    /// Sends a summary with no server (it waits in pending.tsv), then with
    /// the server (both go). Needs DAKKEN_TEST_KEY, a local server on port
    /// 8788 and APPDATA (or HOME) set to a test folder:
    /// `cargo test send_and_queue -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn send_and_queue() {
        let key = std::env::var("DAKKEN_TEST_KEY").expect("DAKKEN_TEST_KEY");
        let account = Account {
            name: "test".into(),
            key,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let run = run_json("words 25", &result(60.0), None, &[], now);
        // SAFETY: the test runs alone (it is ignored and run by name).
        unsafe { std::env::set_var("DAKKEN_API", "http://127.0.0.1:9/api") };
        let line =
            send(account.clone(), Some(run.clone()), now).recv_timeout(Duration::from_secs(20));
        println!("OFFLINE {line:?}");
        let path = pending_path().unwrap();
        println!(
            "PENDING {}",
            fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .count()
        );
        unsafe { std::env::set_var("DAKKEN_API", "http://127.0.0.1:8788/api") };
        let mut ranked = run_json("words 25", &result(61.0), None, &[], now);
        ranked["mode"] = json!("words 50");
        let line = send(account, Some(ranked), now).recv_timeout(Duration::from_secs(20));
        println!("ONLINE {line:?}");
        std::thread::sleep(Duration::from_millis(300));
        println!("PENDING AFTER {}", path.exists());
    }

    /// Sends badges and XP to a local server on port 8788 and prints its
    /// answer. Needs DAKKEN_TEST_KEY: `cargo test send_progress_to_a_server
    /// -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn send_progress_to_a_server() {
        let key = std::env::var("DAKKEN_TEST_KEY").expect("DAKKEN_TEST_KEY");
        let account = Account {
            name: "test".into(),
            key,
        };
        // SAFETY: the test runs alone (it is ignored and run by name).
        unsafe { std::env::set_var("DAKKEN_API", "http://127.0.0.1:8788/api") };
        let badges = [
            ("first press".to_string(), 100),
            ("regular".to_string(), 200),
        ];
        println!(
            "PROGRESS {:?}",
            account::send_progress(&account, &progress_json(1234, &badges))
        );
    }

    #[test]
    fn the_note_names_the_board() {
        let run = json!({ "mode": "time 60 · colemak" });
        assert_eq!(
            note(&run, &json!({ "ranked": true, "best": true, "wpm": 91.6 })).unwrap(),
            "leaderboard: new best ✓ 92 wpm on time 60"
        );
        assert_eq!(note(&run, &json!({ "ranked": false })), None);
    }
}
