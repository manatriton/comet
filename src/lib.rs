mod app;
mod event;
mod network;
mod ui;

use crate::{
    app::{Action, App},
    event::{Event, Events},
};
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use ui::render_page;

pub fn start_ui() -> Result<()> {
    // setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = Events::new(Duration::from_millis(200));

    let mut app = App::new();
    app.dispatch(Action::PageRequest {
        link: String::from("gemini://gemini.circumlunar.space/"),
        push_history: true,
    });

    loop {
        terminal.draw(|f| render_app(f, &mut app))?;
        match events.recv()? {
            Event::Input(event) => {
                if event.code == KeyCode::Char('q') {
                    break;
                }
                if event.code == KeyCode::Enter {
                    app.request_page_from_selected();
                }
                if event.code == KeyCode::Down {
                    app.scroll_down();
                }
                if event.code == KeyCode::Up {
                    app.scroll_up();
                }
                if event.code == KeyCode::Char('f') {
                    if event.modifiers == KeyModifiers::CONTROL {
                        app.page_forward();
                    } else if app.history.has_next() {
                        app.page_next();
                    }
                }
                if event.code == KeyCode::Char('b') {
                    if event.modifiers == (KeyModifiers::CONTROL) {
                        app.page_backward();
                    } else if app.history.has_prev() {
                        app.page_prev();
                    }
                }
                if event.code == KeyCode::Char('j') {
                    app.next_link();
                }
                if event.code == KeyCode::Char('k') {
                    app.previous_link();
                }
                if event.code == KeyCode::Esc {
                    app.clear_highlighted();
                }
            }
            Event::Tick => {
                app.tick()?;
            }
        }
    }

    // restore terminal
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_app<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let block = Block::default().title("Search").borders(Borders::ALL);
    let paragraph = Paragraph::new("gemini://gemini.circumlunar.space/").block(block);
    f.render_widget(paragraph, chunks[0]);

    app.height = chunks[1].height.saturating_sub(2);
    render_page(f, app, chunks[1]);
}
