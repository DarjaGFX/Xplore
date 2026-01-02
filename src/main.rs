mod metadata;
mod filesystem;
mod ui;
mod config;

use std::io;
use std::time::Duration;
use ratatui::{backend::CrosstermBackend, backend::Backend, Terminal};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::ui::app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let res = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> 
where <B as Backend>::Error: 'static
{
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui::ui::render(f, &mut app))?;
        
        // Tick input from PTY
        app.tick();

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                let code_str = match key.code {
                    KeyCode::Char(c) => c.to_string(),
                    _ => String::new(),
                };
                if code_str == app.config.keybindings.quit {
                    return Ok(());
                }
                app.on_key(key.code, key.modifiers);
            }
        }
    }
}
