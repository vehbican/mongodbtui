use crate::{
    app::{ActiveInputField, AppMode, AppState, InputContext},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Text,
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
    theme: &Theme,
) {
    let inner_width = area.width as usize;
    let cursor_position = cursor_position.min(value.graphemes(true).count());
    let cursor_visual_offset = prefix.width()
        + value
            .graphemes(true)
            .take(cursor_position)
            .map(|g| g.width())
            .sum::<usize>();
    let scroll_offset = cursor_visual_offset.saturating_sub(inner_width.saturating_sub(1));
    let command = format!("{prefix}{value}");
    let mut skipped_width = 0;
    let mut visible_width = 0;
    let mut visible = String::new();

    for grapheme in command.graphemes(true) {
        let width = grapheme.width();
        if skipped_width + width <= scroll_offset {
            skipped_width += width;
            continue;
        }

        if visible_width + width > inner_width {
            break;
        }

        visible_width += width;
        visible.push_str(grapheme);
    }

    let paragraph = Paragraph::new(Text::from(visible)).style(Style::default().fg(theme.primary));
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
