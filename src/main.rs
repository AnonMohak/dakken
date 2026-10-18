mod app;
mod engine;
mod ui;

use std::io::{self, Write};
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, terminal,
};

use app::{App, Options};
use engine::Mode;

/// A monkeytype-style typing test for the terminal.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Time mode: test length in seconds. With no mode, the test is untimed.
    #[arg(short, long, value_name = "SECONDS", conflicts_with = "words")]
    time: Option<u64>,

    /// Words mode: number of words.
    #[arg(short, long, value_name = "COUNT")]
    words: Option<usize>,

    /// Add capitals and punctuation to the words.
    #[arg(short, long)]
    punctuation: bool,

    /// Put random numbers in place of some words.
    #[arg(short, long)]
    numbers: bool,

    /// Print each key event, to check how the terminal sends keys.
    #[arg(long)]
    keys: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if cli.keys {
        return key_test();
    }
    let mode = match (cli.time, cli.words) {
        (_, Some(n)) => Mode::Words(n.max(1)),
        (Some(t), None) => Mode::Time(Duration::from_secs(t.max(1))),
        (None, None) => Mode::Untimed,
    };
    let (width, _) = terminal::size()?;
    let options = Options {
        punctuation: cli.punctuation,
        numbers: cli.numbers,
    };
    run(App::new(mode, options, width))
}

fn run(mut app: App) -> io::Result<()> {
    let mut terminal = ratatui::init();
    // ratatui's panic hook restores the terminal. Also give back the cursor shape.
    let restore_terminal = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        restore_terminal(info);
    }));
    execute!(io::stdout(), SetCursorStyle::SteadyBar)?;
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
    ratatui::restore();
    result
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
