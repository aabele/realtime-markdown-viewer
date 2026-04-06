use std::io::{self, stdout, Stdout};
use std::path::PathBuf;

use clap::Parser as ClapParser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod config;
mod event;
mod markdown;
mod theme;
mod ui;
mod watcher;

type Tui = Terminal<CrosstermBackend<Stdout>>;

#[derive(ClapParser)]
#[command(
    name = "rtm",
    version,
    about = "Realtime Markdown - TUI viewer with live reload"
)]
struct Cli {
    path: PathBuf,
}

fn init_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let path = cli.path.canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Directory not found: {}: {}", cli.path.display(), e),
        )
    })?;

    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Not a directory: {}", path.display()),
        ));
    }

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let config = config::Config::load();

    let mut terminal = init_terminal()?;
    let result = event::run(&mut terminal, &path, &config);
    restore_terminal(&mut terminal)?;
    result
}
