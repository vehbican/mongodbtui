use crate::app::{AppMode, AppState};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Paragraph},
};

pub fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let mode_text = match state.mode {
        AppMode::Normal => "[NORMAL]",
        AppMode::Insert => "[INSERT]",
        AppMode::Editor => "[EDITOR]",
    };

    let mut status_line = format!("{}", mode_text);

    if let Some((uri, db, name)) = &state.selected_collection {
        if let Some(count) = state
            .document_counts
            .get(&(uri.clone(), db.clone(), name.clone()))
        {
            status_line.push_str(&format!(" | {}.{} ({} docs)", db, name, count));
        }
    }

    let paragraph = Paragraph::new(Text::from(status_line))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default());

    f.render_widget(paragraph, area);
}
