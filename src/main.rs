mod app;
mod bible;
mod event;
mod translation;
mod tui;
mod ui;

use app::App;
use color_eyre::Result;
use tui::Tui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let terminal = ratatui::init();
    let mut tui = Tui::new(terminal);
    let result = run(&mut tui);
    ratatui::restore();
    result
}

fn run(tui: &mut Tui) -> Result<()> {
    let mut app = App::load()?;

    while app.is_running {
        tui.draw(&app)?;

        match event::next()? {
            event::Event::Key(key) => app.handle_key_event(key),
            event::Event::Resize | event::Event::Tick => app.poll_background_work(),
        }
    }

    Ok(())
}
