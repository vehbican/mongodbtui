use crate::app::{AppMode, InputContext};
use app::AppState;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use db::handler::{fetch_and_update_documents, handle_collection_listing, handle_connection};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::Block,
};
use std::io;
use tui::events::handle_key_event;
use tui::input::render_input;
use widgets::{
    connection_panel::render_connections,
    documents::render_documents,
    header::render_filter,
    help_popup::draw_help_popup,
    import::{centered_rect, render_file_picker},
    popup::{render_popup, render_popup_success},
    toolbar::render_status_bar,
};

mod app;
mod db;
mod keybindings;
mod tui;
mod utils;
mod widgets;

fn apply_cursor_style(state: &AppState) {
    let style = match state.mode {
        AppMode::Insert => SetCursorStyle::SteadyBar,
        AppMode::Normal => SetCursorStyle::SteadyBlock,
    };
    let _ = execute!(io::stdout(), style);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState {
        connections: utils::load_connections().unwrap_or_default(),
        ..Default::default()
    };
    state.rebuild_tree_items();

    apply_cursor_style(&state);

    loop {
        if state.redraw {
            terminal.clear()?;
            state.redraw = false;
        }
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                .split(chunks[0]);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                .split(content_chunks[1]);

            render_connections(f, content_chunks[0], &state);
            render_filter(f, right_chunks[0], &state);
            render_documents(f, right_chunks[1], &state);
            render_status_bar(f, chunks[1], &state);
            render_popup(f, f.area(), &state);
            render_popup_success(f, f.area(), &state);

            if state.mode == AppMode::Insert && state.input_context != InputContext::None {
                render_input(f, f.area(), &mut state);
            }

            if let Some(picker) = &state.file_picker {
                f.render_widget(Block::default(), f.area());
                let popup_area = centered_rect(60, 60, f.area());
                render_file_picker(f, popup_area, picker);
            }

            if state.show_help {
                let area = centered_rect(70, 70, f.area());
                draw_help_popup(f, area, state.help_scroll);
            }
        })?;

        apply_cursor_style(&state);

        if let Some(uri) = state.connect_to.clone() {
            handle_connection(&mut state, &uri).await;
        }

        if let Some((db_uri, db_name)) = state.collection_to_load.clone() {
            handle_collection_listing(&mut state, &db_uri, &db_name).await;
        }

        if let Some((uri, db, name)) = state.fetch_collection_data.clone() {
            fetch_and_update_documents(&mut state, &uri, &db, &name).await;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                let should_exit = handle_key_event(key_event, &mut state).await;
                if should_exit {
                    break;
                }
            }
        }
    }

    let _ = execute!(io::stdout(), SetCursorStyle::DefaultUserShape);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
