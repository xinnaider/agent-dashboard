mod app;
mod cli;
mod detail_ui;
mod model;
mod session;
mod ui;
mod view_ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use app::{App, ViewMode};
use cli::{Cli, Command};

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Json) => {
            let mut app = App::new();
            app.refresh();
            println!("{}", app.to_json());
        }
        Some(Command::View) | None => {
            let start_mode = if matches!(cli.command, Some(Command::View)) {
                ViewMode::View
            } else {
                ViewMode::Table
            };
            run_tui(start_mode)?;
        }
    }

    Ok(())
}

fn run_tui(start_mode: ViewMode) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, start_mode);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_mode: ViewMode,
) -> io::Result<()> {
    let mut app = App::new();
    app.view_mode = start_mode;
    app.refresh();

    let refresh_interval = Duration::from_secs(2);
    let mut last_refresh = Instant::now();

    loop {
        if app.view_mode == ViewMode::View {
            view_ui::resolve_zoom(&mut app);
        }
        terminal.draw(|f| match app.view_mode {
            ViewMode::Table => ui::render(f, &app),
            ViewMode::View => view_ui::render(f, &app),
            ViewMode::Detail => detail_ui::render(f, &app),
        })?;

        app.advance_tick();

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        if last_refresh.elapsed() >= refresh_interval {
            app.refresh();
            last_refresh = Instant::now();
        }
    }
}
