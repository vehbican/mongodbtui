use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::AppState;

pub fn render_editor(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title("Document Editor (Press ESC to exit, Enter to save)")
        .borders(Borders::ALL);

    let lines: Vec<&str> = state.input_text[..state.cursor_position]
        .split('\n')
        .collect();

    let cursor_y = lines.len().saturating_sub(1) as u16;
    let cursor_x = lines.last().map_or(0, |line| line.chars().count()) as u16;

    let visible_height = area.height.saturating_sub(2);
    let scroll_y = cursor_y.saturating_sub(visible_height.saturating_sub(1));

    let paragraph = Paragraph::new(state.input_text.as_str())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_y, 0));

    f.render_widget(paragraph, area);

    f.set_cursor_position((area.x + cursor_x + 1, area.y + cursor_y + 1));
}
