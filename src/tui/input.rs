use crate::app::{AppMode, AppState, ConnectionFormField, InputContext};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

pub fn render_input(f: &mut Frame, area: Rect, state: &mut AppState) {
    let theme = state.theme.palette();
    if state.mode != AppMode::Insert || state.input_context == InputContext::None {
        return;
    }

    if state.input_context == InputContext::NewConnection {
        render_connection_form(f, area, state);
        return;
    }

    let input_area = Rect {
        x: area.width / 3,
        y: area.height / 3,
        width: area.width / 2,
        height: 3,
    };

    let title = match state.input_context {
        InputContext::NewConnection => unreachable!(),
        InputContext::ConnectionName => "Edit Connection Name",
        InputContext::CollectionName => "Rename Collection",
        InputContext::SearchCollections => "Search Collections",
        InputContext::None => unreachable!(),
    };

    state.update_graphemes();

    let inner_width = input_area.width.saturating_sub(2) as usize;
    let cursor_position = state.cursor_position.min(state.input_graphemes.len());
    let cursor_visual_offset = state
        .input_graphemes
        .iter()
        .take(cursor_position)
        .map(|g| g.width())
        .sum::<usize>();

    let scroll_offset = cursor_visual_offset.saturating_sub(inner_width.saturating_sub(1));
    let mut skipped_width = 0;
    let mut visible_width = 0;
    let mut visible_input = String::new();

    for grapheme in &state.input_graphemes {
        let width = grapheme.width();
        if skipped_width + width <= scroll_offset {
            skipped_width += width;
            continue;
        }

        if visible_width + width > inner_width {
            break;
        }

        visible_width += width;
        visible_input.push_str(grapheme);
    }

    let cursor_x_offset = cursor_visual_offset.saturating_sub(skipped_width);

    let input = Paragraph::new(Line::from(Span::raw(visible_input)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().fg(theme.foreground).bg(theme.background)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, input_area);
    f.render_widget(input, input_area);

    f.set_cursor_position((input_area.x + cursor_x_offset as u16 + 1, input_area.y + 1));
}

fn render_connection_form(f: &mut Frame, area: Rect, state: &mut AppState) {
    let theme = state.theme.palette();
    let Some(form) = &state.connection_form else {
        return;
    };
    let width = area.width.saturating_mul(2) / 3;
    let height = 13.min(area.height.saturating_sub(2));
    let form_area = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, form_area);

    let fields = [
        ("Name", &form.name, ConnectionFormField::Name),
        ("URI", &form.uri, ConnectionFormField::Uri),
        ("Color (#RRGGBB)", &form.color, ConnectionFormField::Color),
    ];
    for (index, (label, value, field)) in fields.iter().enumerate() {
        let y = form_area.y + 1 + index as u16 * 3;
        let field_area = Rect {
            x: form_area.x + 2,
            y,
            width: form_area.width.saturating_sub(4),
            height: 3,
        };
        let active = form.active_field == *field;
        let block = Block::default()
            .borders(Borders::ALL)
            .title(*label)
            .border_style(Style::default().fg(if active { theme.accent } else { theme.primary }));
        f.render_widget(Paragraph::new(value.as_str()).block(block), field_area);
        if active {
            let cursor_x =
                field_area.x + 1 + state.cursor_position.min(value.chars().count()) as u16;
            f.set_cursor_position((
                cursor_x.min(field_area.x + field_area.width - 2),
                field_area.y + 1,
            ));
        }
    }

    let footer =
        Paragraph::new("Tab: next field  •  ↑/↓: color presets  •  Enter: save  •  Esc: cancel")
            .style(Style::default().fg(theme.muted));
    f.render_widget(
        footer,
        Rect {
            x: form_area.x + 2,
            y: form_area.y + form_area.height.saturating_sub(1),
            width: form_area.width.saturating_sub(4),
            height: 1,
        },
    );
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("New Connection")
            .border_style(Style::default().fg(theme.accent)),
        form_area,
    );
}
