#![allow(dead_code, unused_variables)]

use std::io;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod app;
mod event;
mod keybindings;
mod theme;
mod ui;

use app::App;
use event::EventReader;

fn main() -> Result<()> {
    // Install panic hook that restores terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let event_reader = EventReader::new(std::time::Duration::from_millis(16));

    // ── Main loop ──────────────────────────────────────────────
    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if let Some(ev) = event_reader.read()? {
            if app.handle_event(ev) {
                break;
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
