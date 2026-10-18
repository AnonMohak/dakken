mod app;
mod config;
mod engine;
mod history;
mod ui;

use std::io::{self, Write};
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, terminal,
};

use app::App;
use config::Config;
use engine::Mode;
use history::History;
use ui::theme::Theme;

/// A monkeytype-style typing test for the terminal.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Time mode: test length in seconds. With no mode, the test is untimed.
    #[arg(short, long, value_name = "SECONDS", conflicts_with_all = ["words", "quote"])]
    time: Option<u64>,

    /// Words mode: number of words.
    #[arg(short, long, value_name = "COUNT", conflicts_with = "quote")]
    words: Option<usize>,

    /// Quote mode: type one quote from a book, poem or speech.
    #[arg(short, long)]
    quote: bool,

    /// Add capitals and punctuation to the words.
    #[arg(short, long)]
    punctuation: bool,

    /// Put random numbers in place of some words.
    #[arg(short, long)]
    numbers: bool,

    /// Color theme: terminal (default), serika-dark, nord or dracula.
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,

    /// Print personal bests and the last tests, then quit.
    #[arg(long)]
    history: bool,

    /// Print each key event, to check how the terminal sends keys.
    #[arg(long)]
    keys: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if cli.keys {
        return key_test();
    }
    let history = history::default_path().map_or_else(History::in_memory, History::load);
    if cli.history {
        print!("{}", history::report(&history));
        return Ok(());
    }
    // The saved settings, with the command-line flags on top.
    let config_path = config::default_path();
    let mut config = config_path
        .as_deref()
        .map_or_else(Config::default, Config::load);
    match (cli.time, cli.words) {
        _ if cli.quote => config.mode = Mode::Quote,
        (_, Some(n)) => config.mode = Mode::Words(n.max(1)),
        (Some(t), None) => config.mode = Mode::Time(Duration::from_secs(t.max(1))),
        (None, None) => {}
    }
    config.options.punctuation |= cli.punctuation;
    config.options.numbers |= cli.numbers;
    if let Some(name) = &cli.theme {
        let Some(theme) = Theme::find(name) else {
            let names = ui::theme::names().join(", ");
            eprintln!("error: unknown theme \"{name}\". Themes: {names}");
            std::process::exit(2);
        };
        config.theme = theme;
    }
    let (width, _) = terminal::size()?;
    run(App::new(config, config_path, width, history))
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
    execute!(io::stdout(), SetCursorStyle::SteadyBlock)?;
    set_wezterm_active(true)?;
    let result = (|| -> io::Result<()> {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &app, Instant::now()))?;
            // About 60 frames per second keeps the timer smooth.
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => app.on_key(key, Instant::now()),
                    Event::Resize(width, _) => app.on_resize(width),
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

/// Tell WezTerm that typer runs, with the `typer_active` user var. A
/// `user-var-changed` handler in the WezTerm config can then make the font
/// bigger. Other terminals do not get the escape code.
fn set_wezterm_active(active: bool) -> io::Result<()> {
    if std::env::var_os("TERM_PROGRAM").is_none_or(|v| v != "WezTerm") {
        return Ok(());
    }
    // The value is base64: "MQ==" is "1" and "MA==" is "0".
    let value = if active { "MQ==" } else { "MA==" };
    let mut out = io::stdout();
    write!(out, "]1337;SetUserVar=typer_active={value}")?;
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
