use crate::{
    app::{ActiveInputField, AppMode, AppState, InputContext},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

fn render_command_line(
    f: &mut Frame,
    area: Rect,
    prefix: &str,
    value: &str,
    cursor_position: usize,
    suggestion: Option<&str>,
    theme: &Theme,
) {
    let inner_width = area.width as usize;
    let cursor_byte = value
        .char_indices()
        .nth(cursor_position)
        .map_or(value.len(), |(i, _)| i);
    let cursor_visual_offset = prefix.width() + value[..cursor_byte].width();
    let scroll_offset = cursor_visual_offset.saturating_sub(inner_width.saturating_sub(1));
    let display_value = suggestion.unwrap_or(value);
    let ghost_start = prefix.len() + cursor_byte;
    let ghost_end = ghost_start + display_value.len().saturating_sub(value.len());
    let command = format!("{prefix}{display_value}");
    let mut skipped_width = 0;
    let mut visible_width = 0;
    let mut visible = Vec::new();

    for (byte, grapheme) in command.grapheme_indices(true) {
        let width = grapheme.width();
        if skipped_width + width <= scroll_offset {
            skipped_width += width;
            continue;
        }

        if visible_width + width > inner_width {
            break;
        }

        visible_width += width;
        let color = if byte >= ghost_start && byte < ghost_end {
            theme.muted
        } else {
            theme.primary
        };
        visible.push(Span::styled(grapheme, Style::default().fg(color)));
    }

    let paragraph = Paragraph::new(Line::from(visible));
    f.render_widget(paragraph, area);

    let cursor_x = area.x + cursor_visual_offset.saturating_sub(skipped_width) as u16;
    f.set_cursor_position((cursor_x.min(area.x + area.width.saturating_sub(1)), area.y));
}

pub fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = state.theme.palette();
    if state.mode == AppMode::Insert && state.input_context == InputContext::None {
        match state.active_input {
            Some(ActiveInputField::Filter) => {
                render_command_line(
                    f,
                    area,
                    "/",
                    &state.filter_text,
                    state.cursor_position,
                    state.query_suggestion(),
                    &theme,
                );
                return;
            }
            Some(ActiveInputField::Sort) => {
                render_command_line(
                    f,
                    area,
                    ":sort ",
                    &state.sort_text,
                    state.cursor_position,
                    state.query_suggestion(),
                    &theme,
                );
                return;
            }
            _ => {}
        }
    }

    let mode_text = match state.mode {
        AppMode::Normal => "[NORMAL]",
        AppMode::Insert => "[INSERT]",
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
        .style(Style::default().fg(theme.secondary))
        .block(Block::default());

    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn suggestion_is_muted_and_keeps_the_closing_brace_and_cursor() {
        let mut state = AppState {
            mode: AppMode::Insert,
            filter_text: "{\"n}".to_owned(),
            cursor_position: 3,
            ..Default::default()
        };
        state
            .query_history
            .record(ActiveInputField::Filter, "{\"name\": 1}");
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).unwrap();
        let frame = terminal
            .draw(|f| render_status_bar(f, f.area(), &state))
            .unwrap();
        let theme = state.theme.palette();
        assert_eq!(frame.buffer[(3, 0)].symbol(), "n");
        assert_eq!(frame.buffer[(3, 0)].fg, theme.primary);
        assert_eq!(frame.buffer[(4, 0)].symbol(), "a");
        assert_eq!(frame.buffer[(4, 0)].fg, theme.muted);
        assert_eq!(frame.buffer[(11, 0)].symbol(), "}");
        assert_eq!(frame.buffer[(11, 0)].fg, theme.primary);
        assert_eq!(state.cursor_position, 3);
    }
}
