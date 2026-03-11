use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};

/// Application-level event, decoupled from crossterm for future extensibility.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventReader {
    tick_rate: Duration,
}

impl EventReader {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Poll for the next event. Returns `None` on tick (no input within tick_rate).
    pub fn read(&self) -> Result<Option<AppEvent>> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(Some(AppEvent::Key(key))),
                Event::Mouse(mouse) => Ok(Some(AppEvent::Mouse(mouse))),
                Event::Resize(w, h) => Ok(Some(AppEvent::Resize(w, h))),
                // FocusGained, FocusLost, Paste — ignored for now
                _ => Ok(None),
            }
        } else {
            Ok(None) // tick
        }
    }
}
