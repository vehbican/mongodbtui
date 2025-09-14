use crate::app::{AppMode, AppState, InputContext};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

pub fn render_input(f: &mut Frame, area: Rect, state: &mut AppState) {
    if state.mode != AppMode::Insert || state.input_context == InputContext::None {
        return;
    }

    let input_area = Rect {
        x: area.width / 3,
        y: area.height / 3,
        width: area.width / 2,
        height: 3,
    };

    let title = match state.input_context {
        InputContext::Uri => "Enter Mongo URI",
        InputContext::ConnectionName => "Edit Connection Name",
        InputContext::CollectionName => "Rename Collection",
        InputContext::None => unreachable!(),
    };

    state.update_graphemes();

    let cursor_visual_offset = state
        .input_graphemes
        .iter()
        .take(state.cursor_position)
        .map(|g| g.width())
        .sum::<usize>();

    let input = Paragraph::new(Line::from(Span::raw(&state.input_text)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Green))
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, input_area);
    f.render_widget(input, input_area);

    f.set_cursor_position((
        input_area.x + cursor_visual_offset as u16 + 1,
        input_area.y + 1,
    ));
}
