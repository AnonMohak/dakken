mod app;
mod config;
mod engine;
mod history;
mod keystats;
mod layout;
mod mistakes;
mod ui;
mod update;

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
use config::{Caret, Config};
use engine::words::{WORD_LISTS, WordList};
use engine::{Mode, QuoteLength, STRICT_MODES, Strict};
use history::History;
use keystats::KeyStats;
use mistakes::Mistakes;
use ui::theme::Theme;

/// A typing-speed test for the terminal.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Time mode: test length in seconds. With no mode, the test is untimed.
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

    /// Add capitals and punctuation to the words.
    #[arg(short, long)]
    punctuation: bool,

    /// Put random numbers in place of some words.
    #[arg(short, long)]
    numbers: bool,

    /// Word list: english (default), english-1k, code, spanish, german,
    /// mistakes or weak-keys.
    #[arg(short = 'l', long, value_name = "NAME")]
    words_list: Option<String>,

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

    /// Color theme: terminal (default), serika-dark, nord, dracula, or custom
    /// (from the color_* keys in config.toml).
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

    /// Ask GitHub if there is a newer release, then quit. Needs curl.
    #[arg(long)]
    check_update: bool,

    /// Print the full history as csv or json, then quit.
    #[arg(long, value_name = "FORMAT", value_parser = ["csv", "json"])]
    export: Option<String>,

    /// Print each key event, to check how the terminal sends keys.
    #[arg(long)]
    keys: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
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
        return Ok(());
    }
    let key_stats = keystats::default_path().map_or_else(KeyStats::in_memory, KeyStats::load);
    if cli.heatmap {
        use std::io::IsTerminal;
        let color = io::stdout().is_terminal();
        print!("{}", keystats::heatmap(&key_stats, color));
        print!(
            "\n{}",
            history::day_heatmap(&history, history::today(), color)
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
    config.options.punctuation |= cli.punctuation;
    config.options.numbers |= cli.numbers;
    if let Some(name) = &cli.words_list {
        let Some(list) = WordList::parse(name) else {
            let names: Vec<&str> = WORD_LISTS.iter().map(|l| l.name()).collect();
            eprintln!(
                "error: unknown word list \"{name}\". Lists: {}",
                names.join(", ")
            );
            std::process::exit(2);
        };
        config.options.list = list;
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
    if cli.daily {
        app.start_daily(history::today());
    }
    run(app)
}

fn run(mut app: App) -> io::Result<()> {
    let mut terminal = ratatui::init();
    // ratatui's panic hook restores the terminal. Also give back the cursor shape.
    let restore_terminal = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        let _ = set_wezterm_active(false);
        restore_terminal(info);
    }));
    let caret = match app.caret {
        Caret::Block => SetCursorStyle::SteadyBlock,
        Caret::Bar => SetCursorStyle::SteadyBar,
        Caret::Underline => SetCursorStyle::SteadyUnderScore,
    };
    execute!(io::stdout(), caret)?;
    set_wezterm_active(true)?;
    let result = (|| -> io::Result<()> {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &app, Instant::now()))?;
            // About 60 frames per second keeps the timer smooth.
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => app.on_key(key, Instant::now()),
                    Event::Resize(width, height) => app.on_resize(width, height),
                    _ => {}
                }
            }
            app.tick(Instant::now());
        }
        Ok(())
    })();
    execute!(io::stdout(), SetCursorStyle::DefaultUserShape)?;
    set_wezterm_active(false)?;
    ratatui::restore();
    result
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
