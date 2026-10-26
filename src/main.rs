mod account;
mod app;
mod badges;
mod best_runs;
mod card;
mod challenge;
mod clipboard;
mod config;
mod engine;
mod history;
mod keystats;
mod layout;
mod localtime;
mod mistakes;
mod practice;
mod progress;
mod quests;
mod race;
mod sha256;
mod ui;
mod update;
mod upload;

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, terminal,
};

use app::{App, CustomText};
use badges::Badges;
use best_runs::BestRuns;
use config::{Caret, Config};
use engine::words::{WORD_LISTS, WordList};
use engine::{Mode, PracticeKind, QuoteLength, STRICT_MODES, Strict};
use history::History;
use keystats::KeyStats;
use mistakes::Mistakes;
use practice::Progress;
use quests::Quests;
use ui::theme::Theme;

/// A typing-speed test for the terminal.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Time mode: test length in seconds. With no mode, the test is 30 seconds.
    #[arg(short, long, value_name = "SECONDS", conflicts_with_all = ["words", "quote"])]
    time: Option<u64>,

    /// Words mode: number of words.
    #[arg(short, long, value_name = "COUNT", conflicts_with = "quote")]
    words: Option<usize>,

    /// Quote mode: type one quote from a book, poem or speech. LENGTH is
    /// all (default), short, medium or long.
    #[arg(short, long, value_name = "LENGTH", num_args = 0..=1, default_missing_value = "all")]
    quote: Option<String>,

    /// Custom mode: type the text of this file.
    #[arg(short, long, value_name = "PATH", conflicts_with_all = ["time", "words", "quote"])]
    file: Option<PathBuf>,

    /// Zen mode: type anything with no target text. Enter ends the test.
    #[arg(short, long, conflicts_with_all = ["time", "words", "quote", "file"])]
    zen: bool,

    /// Daily challenge: the same 30 words for everyone today. Your streak
    /// shows in --stats.
    #[arg(short, long, conflicts_with_all = ["time", "words", "quote", "file", "zen"])]
    daily: bool,

    /// Race the ghost of a race file, saved with x on the results.
    #[arg(long, value_name = "PATH", conflicts_with_all = ["time", "words", "quote", "file", "zen", "daily"])]
    race: Option<PathBuf>,

    /// Adaptive practice: made-up words from a few letters, and a new letter
    /// when they are fast enough. Practice is not saved in the history.
    #[arg(long, conflicts_with_all = ["time", "words", "quote", "file", "zen", "daily", "race"])]
    practice: bool,

    /// Add capitals and punctuation to the words.
    #[arg(short, long)]
    punctuation: bool,

    /// Put random numbers in place of some words.
    #[arg(short, long)]
    numbers: bool,

    /// Word list: english (default), english-1k, code, spanish, german,
    /// french or portuguese. mistakes, weak-keys and weak-pairs start that
    /// kind of practice.
    #[arg(short = 'l', long, value_name = "NAME")]
    words_list: Option<String>,

    /// Use the words of this file as the word list, in random order (with
    /// --file, the text is typed in order instead).
    #[arg(long, value_name = "PATH", conflicts_with_all = ["file", "quote", "zen", "daily", "race", "words_list"])]
    words_file: Option<PathBuf>,

    /// Caret shape: block (default), bar or underline.
    #[arg(long, value_name = "SHAPE", value_parser = ["block", "bar", "underline"])]
    caret: Option<String>,

    /// Keyboard layout to practise on a QWERTY keyboard: qwerty (default),
    /// dvorak or colemak.
    #[arg(long, value_name = "NAME", value_parser = ["qwerty", "dvorak", "colemak"])]
    layout: Option<String>,

    /// Strict mode: off (default), stop-on-error, no-backspace or master.
    #[arg(long, value_name = "NAME")]
    strict: Option<String>,

    /// Challenge: off (default), survival, boss or climb.
    #[arg(short = 'g', long, value_name = "NAME")]
    challenge: Option<String>,

    /// Color theme: sumi (default), tokyo-night, terminal, dracula, serika-dark,
    /// nord, catppuccin-mocha, rose-pine, one-dark, blueprint, matcha, abyss,
    /// synthwave, catppuccin-latte, github-light, or custom (from the
    /// color_* keys in config.toml).
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,

    /// Print personal bests and the last tests, then quit.
    #[arg(long)]
    history: bool,

    /// Print averages for each mode and a graph of WPM by day, then quit.
    #[arg(long)]
    stats: bool,

    /// Print a keyboard colored by errors, the slowest keys, and a calendar
    /// of your speed by day, then quit.
    #[arg(long)]
    heatmap: bool,

    /// Ask GitHub if there is a newer release, then quit. Needs curl. The
    /// app also asks once a day and shows a newer version in the menu
    /// (`update_check = false` in config.toml turns this off).
    #[arg(long)]
    check_update: bool,

    /// Print the full history as csv or json, then quit.
    #[arg(long, value_name = "FORMAT", value_parser = ["csv", "json"])]
    export: Option<String>,

    /// Print the level of each key and your key quests, then quit.
    #[arg(long)]
    skills: bool,

    /// Print your level, XP and badges, then quit.
    #[arg(long)]
    badges: bool,

    /// Print each key event, to check how the terminal sends keys.
    #[arg(long)]
    keys: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

/// The leaderboard profile at me-momo.co.in/dakken.
#[derive(clap::Subcommand)]
enum Command {
    /// Make your leaderboard profile with this name, or change its name.
    Name { name: String },
    /// Without a code: show a code that links one more device to your
    /// profile. With a code: join the profile of another device.
    Link { code: Option<String> },
    /// Get your profile back with its recovery code. A new code replaces it.
    Recover { code: String },
    /// Show your profile and its devices.
    Account,
}

const BOARD: &str = "https://me-momo.co.in/dakken";

/// Run a profile command. An error goes to stderr with exit code 1.
fn profile_command(command: Command) -> io::Result<()> {
    let result = match command {
        Command::Name { name } => name_command(&name),
        Command::Link { code: None } => link_command(),
        Command::Link { code: Some(code) } => join_command(&code),
        Command::Recover { code } => recover_command(&code),
        Command::Account => account_command(),
    };
    if let Err(e) = result {
        eprintln!("dakken: {e}");
        std::process::exit(1);
    }
    Ok(())
}

fn keep(account: &account::Account) -> Result<(), String> {
    account::save(account).map_err(|e| format!("cannot save the profile on this computer: {e}"))
}

fn show_recovery(code: &str) {
    println!("recovery code: {code}");
    println!("Keep it somewhere safe. It is shown only now, and it gives your profile");
    println!("back if this computer's data is lost (`dakken recover <code>`).");
}

fn name_command(name: &str) -> Result<(), String> {
    if let Some(mut account) = account::load() {
        account.name = account::rename(&account, name)?;
        keep(&account)?;
        println!("your name is now {}", account.name);
        return Ok(());
    }
    let (account, code) = account::create(name)?;
    keep(&account)?;
    println!(
        "you are {} on the leaderboard: {BOARD}/leaderboard/{}/",
        account.name, account.name
    );
    println!("Your time 30 and time 60 runs now go to the boards.");
    println!();
    show_recovery(&code);
    Ok(())
}

fn link_command() -> Result<(), String> {
    let account =
        account::load().ok_or("no profile on this computer; make one with `dakken name <name>`")?;
    let (code, _) = account::link_code(&account)?;
    println!("link code: {code} (it works one time, for 10 minutes)");
    println!("On the other device, run `dakken link {code}`,");
    println!("or open {BOARD}/profile/ and use \"already have a profile?\".");
    Ok(())
}

fn join_command(code: &str) -> Result<(), String> {
    if let Some(account) = account::load() {
        return Err(format!(
            "this computer already has the profile {}",
            account.name
        ));
    }
    let account = account::join(code)?;
    keep(&account)?;
    println!("this computer is now {} on the leaderboard", account.name);
    Ok(())
}

fn recover_command(code: &str) -> Result<(), String> {
    let (account, new_code) = account::recover(code)?;
    keep(&account)?;
    println!(
        "your profile {} is back on this computer. Your old recovery code stopped.",
        account.name
    );
    println!();
    show_recovery(&new_code);
    Ok(())
}

fn account_command() -> Result<(), String> {
    let Some(account) = account::load() else {
        println!("no profile on this computer. Make one with `dakken name <name>`,");
        println!("or join one with `dakken link <code>`.");
        return Ok(());
    };
    println!("{} · {BOARD}/leaderboard/{}/", account.name, account.name);
    println!();
    println!("devices:");
    for (label, this) in account::devices(&account)? {
        println!("  {label}{}", if this { " (this one)" } else { "" });
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut cli = Cli::parse();
    if let Some(command) = cli.command.take() {
        return profile_command(command);
    }
    if cli.keys {
        return key_test();
    }
    if cli.check_update {
        print!("{}", update::check());
        return Ok(());
    }
    let history = history::default_path().map_or_else(History::in_memory, History::load);
    if cli.history {
        print!("{}", history::report(&history));
        return Ok(());
    }
    match cli.export.as_deref() {
        Some("csv") => {
            print!("{}", history::to_csv(&history));
            return Ok(());
        }
        Some(_) => {
            print!("{}", history::to_json(&history));
            return Ok(());
        }
        None => {}
    }
    if cli.stats {
        print!("{}", history::stats_report(&history));
        let goal = config::default_path().map_or(0, |p| Config::load(&p).weekly_goal);
        print!(
            "{}",
            progress::weekly_report(history.records(), goal, history::today())
        );
        return Ok(());
    }
    let mut badges = badges::default_path().map_or_else(Badges::in_memory, Badges::load);
    if cli.badges {
        // Tests from before badges existed can earn them too.
        let goal = config::default_path().map_or(0, |p| Config::load(&p).weekly_goal);
        let _ = badges
            .add(&badges::earned(
                history.records(),
                &badges::Moment::default(),
                goal,
            ))
            .1;
        let color = color_output();
        print!(
            "{}",
            themed(&badges::report(&history, &badges, color), &cli)
        );
        return Ok(());
    }
    let key_stats = keystats::default_path().map_or_else(KeyStats::in_memory, KeyStats::load);
    if cli.heatmap {
        let color = color_output();
        print!("{}", themed(&keystats::heatmap(&key_stats, color), &cli));
        let days = history::day_heatmap(&history, history::today(), color);
        print!("\n{}", themed(&days, &cli));
        return Ok(());
    }
    let mut quests = quests::default_path().map_or_else(Quests::in_memory, Quests::load);
    if cli.skills {
        let color = color_output();
        let now = history.records().last().map_or(0, |r| r.time);
        let _ = quests.fill(&key_stats, now);
        print!(
            "{}",
            themed(&quests::skill_map(&key_stats, &quests, color), &cli)
        );
        return Ok(());
    }
    // The saved settings, with the command-line flags on top.
    let config_path = config::default_path();
    let mut config = config_path
        .as_deref()
        .map_or_else(Config::default, Config::load);
    if let Some(name) = &cli.quote {
        let Some(length) = QuoteLength::parse(name) else {
            eprintln!("error: unknown quote length \"{name}\". Lengths: all, short, medium, long");
            std::process::exit(2);
        };
        config.mode = Mode::Quote(length);
    }
    match (cli.time, cli.words) {
        _ if cli.quote.is_some() => {}
        _ if cli.zen => config.mode = Mode::Zen,
        (_, Some(n)) => config.mode = Mode::Words(n.max(1)),
        (Some(t), None) => config.mode = Mode::Time(Duration::from_secs(t.max(1))),
        (None, None) => {}
    }
    if cli.practice {
        config.mode = Mode::Practice(PracticeKind::Adaptive);
    }
    config.options.punctuation |= cli.punctuation;
    config.options.numbers |= cli.numbers;
    if let Some(name) = &cli.words_list {
        // The weak lists of old versions are practice kinds now.
        if let Some(kind) = PracticeKind::parse(name).filter(|&k| k != PracticeKind::Adaptive) {
            config.mode = Mode::Practice(kind);
        } else if let Some(list) = WordList::parse(name) {
            config.options.list = list;
        } else {
            let names: Vec<&str> = WORD_LISTS.iter().map(|l| l.name()).collect();
            eprintln!(
                "error: unknown word list \"{name}\". Lists: {}",
                names.join(", ")
            );
            std::process::exit(2);
        }
    }
    if let Some(caret) = cli.caret.as_deref().and_then(Caret::parse) {
        config.caret = caret;
    }
    if let Some(layout) = cli.layout.as_deref().and_then(layout::Layout::parse) {
        config.layout = layout;
    }
    if let Some(name) = &cli.strict {
        let Some(strict) = Strict::parse(name) else {
            let names: Vec<&str> = STRICT_MODES.iter().map(|s| s.name()).collect();
            eprintln!(
                "error: unknown strict mode \"{name}\". Modes: {}",
                names.join(", ")
            );
            std::process::exit(2);
        };
        config.strict = strict;
    }
    if let Some(name) = &cli.challenge {
        let Some(c) = challenge::Challenge::parse(name) else {
            let names: Vec<&str> = challenge::CHALLENGES.iter().map(|c| c.name()).collect();
            eprintln!(
                "error: unknown challenge \"{name}\". Challenges: {}",
                names.join(", ")
            );
            std::process::exit(2);
        };
        config.challenge = c;
    }
    if let Some(name) = &cli.theme {
        let custom = config.custom_theme.filter(|_| name == "custom");
        let Some(theme) = Theme::find(name).or(custom) else {
            let mut names = ui::theme::names();
            if config.custom_theme.is_some() {
                names.push("custom");
            }
            let names = names.join(", ");
            eprintln!("error: unknown theme \"{name}\". Themes: {names}");
            std::process::exit(2);
        };
        config.theme = theme;
    }
    let size = terminal::size()?;
    let mistakes = mistakes::default_path().map_or_else(Mistakes::in_memory, Mistakes::load);
    let custom = match &cli.file {
        Some(path) => match CustomText::load(path) {
            Ok(custom) => Some(custom),
            Err(e) => {
                eprintln!("error: cannot use {}: {e}", path.display());
                std::process::exit(2);
            }
        },
        None => None,
    };
    let mut app = App::new(
        config,
        config_path,
        size,
        history,
        mistakes,
        key_stats,
        custom,
    );
    // With a leaderboard profile, finished tests go to the server.
    app.account = account::load();
    if let Some(path) = &cli.words_file {
        match CustomText::load(path) {
            Ok(mut file) => {
                file.words.sort();
                file.words.dedup();
                app.word_file = Some(file);
                app.new_test();
            }
            Err(e) => {
                eprintln!("error: cannot use {}: {e}", path.display());
                std::process::exit(2);
            }
        }
    }
    app.best_runs = best_runs::default_path().map_or_else(BestRuns::in_memory, BestRuns::load);
    app.badges = badges;
    app.quests = quests;
    // Practice keeps its own data, apart from the tests. The first lesson
    // needs it and the quests, so it is made again.
    app.practice = practice::default_path().map_or_else(Progress::default, Progress::load);
    app.practice_keys = practice::keys_path().map_or_else(KeyStats::in_memory, KeyStats::load);
    app.practice_mistakes =
        practice::mistakes_path().map_or_else(Mistakes::in_memory, Mistakes::load);
    if app.practising() {
        app.new_test();
    }
    if cli.daily {
        app.start_daily(history::today());
    }
    if let Some(path) = &cli.race {
        match race::Race::load(path) {
            Ok(race) => app.start_race(race),
            Err(e) => {
                eprintln!("error: cannot race {}: {e}", path.display());
                std::process::exit(2);
            }
        }
    }
    // Leftover from a self-update on Windows, which cannot delete a running
    // program.
    update::remove_old_exe();
    let updates = app.update_check.then(update::check_in_background);
    if let Some(tag) = run(app, updates)? {
        print!("{}", update::update(&tag));
    }
    Ok(())
}

/// Run the app. Gives the newer release tag if the user pressed `u` to
/// update.
fn run(
    mut app: App,
    updates: Option<std::sync::mpsc::Receiver<String>>,
) -> io::Result<Option<String>> {
    let mut terminal = ratatui::init();
    // ratatui's panic hook restores the terminal. Also give back the cursor shape.
    let restore_terminal = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        let _ = set_wezterm_active(false);
        restore_terminal(info);
    }));
    let mut caret = app.caret;
    execute!(io::stdout(), cursor_style(caret))?;
    set_wezterm_active(true)?;
    // The system clipboard command runs on its own thread, so a slow start
    // (PowerShell on Windows) does not stop the screen.
    let (copied_tx, copied) = std::sync::mpsc::channel();
    let result = (|| -> io::Result<()> {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &app, Instant::now()))?;
            // About 60 frames per second keeps the timer smooth.
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => {
                        app.on_key(key, Instant::now());
                        // The settings screen can change the caret.
                        if app.caret != caret {
                            caret = app.caret;
                            execute!(io::stdout(), cursor_style(caret))?;
                        }
                        if let Some(text) = app.copy.take() {
                            let mut out = io::stdout();
                            write!(out, "{}", card::osc52(&text))?;
                            out.flush()?;
                            let tx = copied_tx.clone();
                            std::thread::spawn(move || tx.send(clipboard::copy(&text)));
                        }
                    }
                    Event::Resize(width, height) => app.on_resize(width, height),
                    _ => {}
                }
            }
            app.tick(Instant::now());
            if let Ok(ok) = copied.try_recv() {
                app.copied |= ok;
            }
            if let Some(tag) = updates.as_ref().and_then(|u| u.try_recv().ok()) {
                app.update = Some(tag);
            }
        }
        Ok(())
    })();
    execute!(io::stdout(), SetCursorStyle::DefaultUserShape)?;
    set_wezterm_active(false)?;
    ratatui::restore();
    result.map(|()| app.update.filter(|_| app.update_now))
}

fn cursor_style(caret: Caret) -> SetCursorStyle {
    match caret {
        Caret::Block => SetCursorStyle::SteadyBlock,
        Caret::Bar => SetCursorStyle::SteadyBar,
        Caret::Underline => SetCursorStyle::SteadyUnderScore,
    }
}

/// A printed report in the colors of the theme: the saved one, or the one
/// of `--theme`.
fn themed(report: &str, cli: &Cli) -> String {
    let config = config::default_path().map_or_else(Config::default, |p| Config::load(&p));
    let custom = config
        .custom_theme
        .filter(|_| cli.theme.as_deref() == Some("custom"));
    let theme = (cli.theme.as_deref().and_then(Theme::find))
        .or(custom)
        .unwrap_or(config.theme);
    ui::themed_report(report, theme, ui::color256::truecolor())
}

/// Color in the printed reports: only in a terminal, and not with the
/// `NO_COLOR` environment variable (no-color.org).
fn color_output() -> bool {
    use std::io::IsTerminal;
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

/// Tell WezTerm that dakken runs, with the `dakken_active` user var. A
/// `user-var-changed` handler in the WezTerm config can then make the font
/// bigger. Other terminals do not get the escape code.
fn set_wezterm_active(active: bool) -> io::Result<()> {
    if std::env::var_os("TERM_PROGRAM").is_none_or(|v| v != "WezTerm") {
        return Ok(());
    }
    // The value is base64: "MQ==" is "1" and "MA==" is "0".
    let value = if active { "MQ==" } else { "MA==" };
    let mut out = io::stdout();
    write!(out, "\x1b]1337;SetUserVar=dakken_active={value}\x07")?;
    out.flush()
}

/// Print every key event in raw mode. Ctrl+C quits.
fn key_test() -> io::Result<()> {
    println!("Press keys to see their events. Press Ctrl+C to quit.");
    terminal::enable_raw_mode()?;
    let result = (|| -> io::Result<()> {
        let mut out = io::stdout();
        loop {
            if let Event::Key(key) = event::read()? {
                write!(
                    out,
                    "{:<8} code={:?} modifiers={:?}\r\n",
                    format!("{:?}", key.kind),
                    key.code,
                    key.modifiers
                )?;
                out.flush()?;
                if key.kind == KeyEventKind::Press
                    && key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    return Ok(());
                }
            }
        }
    })();
    terminal::disable_raw_mode()?;
    result
}
