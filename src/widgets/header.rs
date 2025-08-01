use crate::app::{ActiveInputField, AppMode, AppState, FocusArea};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn render_filter(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let filter = Paragraph::new(Text::from(state.filter_text.clone()))
        .block(
            Block::default()
                .title("🔍 Filter")
                .borders(Borders::ALL)
                .border_style(
                    if state.focus == FocusArea::FilterSortInputs
                        && state.active_input == Some(ActiveInputField::Filter)
                    {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                )
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(filter, chunks[0]);

    let sort = Paragraph::new(Text::from(state.sort_text.clone()))
        .block(
            Block::default()
                .title("⇅ Sort")
                .borders(Borders::ALL)
                .border_style(
                    if state.focus == FocusArea::FilterSortInputs
                        && state.active_input == Some(ActiveInputField::Sort)
                    {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                )
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(sort, chunks[1]);

    if state.mode == AppMode::Insert && state.focus == FocusArea::FilterSortInputs {
        match state.active_input {
            Some(ActiveInputField::Filter) => {
                let offset = state
                    .filter_text
                    .graphemes(true)
                    .take(state.cursor_position)
                    .map(|g| g.width())
                    .sum::<usize>();

                f.set_cursor_position((chunks[0].x + offset as u16 + 1, chunks[0].y + 1));
            }
            Some(ActiveInputField::Sort) => {
                let offset = state
                    .sort_text
                    .graphemes(true)
                    .take(state.cursor_position)
                    .map(|g| g.width())
                    .sum::<usize>();

                f.set_cursor_position((chunks[1].x + offset as u16 + 1, chunks[1].y + 1));
            }
            _ => {}
        }
    }
}
