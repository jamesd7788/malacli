use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};

pub enum Event {
    Key(KeyEvent),
    Resize,
    Tick,
}

pub fn next() -> Result<Event> {
    loop {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                CrosstermEvent::Key(key) => return Ok(Event::Key(key)),
                CrosstermEvent::Resize(_, _) => return Ok(Event::Resize),
                _ => {}
            }
        } else {
            return Ok(Event::Tick);
        }
    }
}
