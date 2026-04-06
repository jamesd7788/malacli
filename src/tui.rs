use color_eyre::Result;
use ratatui::{DefaultTerminal, Frame};

use crate::{app::App, ui};

pub struct Tui {
    terminal: DefaultTerminal,
}

impl Tui {
    pub fn new(terminal: DefaultTerminal) -> Self {
        Self { terminal }
    }

    pub fn draw(&mut self, app: &App) -> Result<()> {
        self.terminal.draw(|frame| render(frame, app))?;
        Ok(())
    }
}

fn render(frame: &mut Frame<'_>, app: &App) {
    ui::render(frame, app);
}
