mod app;
mod screens;
mod terminal;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};

fn main() -> io::Result<()> {
    terminal::install_panic_hook();
    let mut tui = terminal::init()?;
    let mut app = app::App::new();

    let target_frame = Duration::from_millis(16); // ~60 fps cap
    let mut last_tick = Instant::now();

    loop {
        // Advance simulation by real elapsed time since the last frame.
        let delta = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();
        app.tick(delta);

        tui.draw(|frame| screens::render(frame, &app))?;

        // Poll for input without blocking longer than one frame.
        if event::poll(target_frame)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    terminal::restore()?;
    Ok(())
}
