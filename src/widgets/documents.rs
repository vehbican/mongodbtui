use crate::{
    app::{AppState, FocusArea},
    theme::Theme,
};
use bson::Bson;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

const FIELD_PREVIEW_CHAR_LIMIT: usize = 160;

fn format_bson_value(value: &Bson) -> String {
    match value {
        Bson::ObjectId(oid) => format!("ObjectId(\"{}\")", oid),
        Bson::DateTime(dt) => dt.to_string(),
        Bson::Document(inner) => match serde_json::to_string_pretty(inner) {
            Ok(s) => s,
            Err(_) => "<invalid document>".to_string(),
        },
        _ => format!("{}", value),
    }
}

fn preview_value(value: &str) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= FIELD_PREVIEW_CHAR_LIMIT {
        return single_line;
    }

    format!(
        "{} [truncated - press Enter to expand]",
        single_line
            .chars()
            .take(FIELD_PREVIEW_CHAR_LIMIT)
            .collect::<String>()
    )
}

fn push_field_lines(
    lines: &mut Vec<Line<'static>>,
    key: &str,
    value: &str,
    is_selected: bool,
    is_expanded: bool,
    theme: &Theme,
) {
    let style = Style::default()
        .fg(theme.accent)
        .bg(theme.primary)
        .add_modifier(Modifier::BOLD);

    if !is_expanded {
        let formatted = format!("{:<15}: {}", key, preview_value(value));
        if is_selected {
            lines.push(Line::from(vec![Span::styled(formatted, style)]));
        } else {
            lines.push(Line::from(formatted));
        }
        return;
    }

    for (line_index, value_line) in value.lines().enumerate() {
        let formatted = if line_index == 0 {
            format!("{:<15}: {}", key, value_line)
        } else {
            format!("{:<15}  {}", "", value_line)
        };

        if is_selected {
            lines.push(Line::from(vec![Span::styled(formatted, style)]));
        } else {
            lines.push(Line::from(formatted));
        }
    }
}

pub fn render_documents(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = state.theme.palette();
    let [tab_area, documents_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .areas(area);

    if !state.collection_tabs.is_empty() {
        let titles = state
            .collection_tabs
            .iter()
            .map(|tab| {
                let connection_name = state
                    .connections
                    .iter()
                    .find(|connection| connection.uri == tab.key.0)
                    .map(|connection| connection.name.as_str())
                    .unwrap_or(tab.key.0.as_str());
                format!(" {} / {}.{} ", connection_name, tab.key.1, tab.key.2)
            })
            .collect::<Vec<_>>();
        let separator_width = " │ ".width();
        let left_marker_width = "… ".width();
        let right_marker_width = " …".width();
        let available_width = tab_area.width as usize;

        // Keep the active tab visible. When the tab row overflows, move its
        // left edge just far enough to bring the active tab into view.
        let mut first = 0;
        while first < state.active_tab {
            let width_to_active = titles[first..=state.active_tab]
                .iter()
                .map(|title| title.width())
                .sum::<usize>()
                + separator_width * (state.active_tab - first);
            if width_to_active + if first > 0 { left_marker_width } else { 0 } <= available_width {
                break;
            }
            first += 1;
        }

        let has_left = first > 0;
        let mut used_width = if has_left { left_marker_width } else { 0 };
        let mut last = first;
        while last < titles.len() {
            let item_width = titles[last].width() + if last > first { separator_width } else { 0 };
            let reserve_right = if last + 1 < titles.len() {
                right_marker_width
            } else {
                0
            };
            if last > state.active_tab && used_width + item_width + reserve_right > available_width
            {
                break;
            }
            if used_width + item_width > available_width {
                break;
            }
            used_width += item_width;
            last += 1;
        }

        let mut tab_spans = Vec::new();
        if has_left {
            tab_spans.push(Span::styled("… ", Style::default().fg(theme.muted)));
        }
        for index in first..last {
            if index > first {
                tab_spans.push(Span::styled(" │ ", Style::default().fg(theme.muted)));
            }

            let title = &titles[index];
            let style = if index == state.active_tab {
                Style::default()
                    .fg(theme.primary)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.secondary)
            };
            tab_spans.push(Span::styled(title.clone(), style));
        }
        if last < titles.len() {
            tab_spans.push(Span::styled(" …", Style::default().fg(theme.muted)));
        }
        f.render_widget(Paragraph::new(Line::from(tab_spans)), tab_area);
    }

    let documents = &state.current_documents;

    if documents.is_empty() {
        let block = Block::default()
            .title("Documents")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.secondary));

        let empty = Paragraph::new("No documents to display")
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(empty, documents_area);
        return;
    }

    let max_visible = 1;

    let visible_docs = documents
        .iter()
        .skip(state.doc_scroll_offset)
        .take(max_visible)
        .collect::<Vec<_>>();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            visible_docs
                .iter()
                .map(|_| Constraint::Length(80))
                .collect::<Vec<_>>(),
        )
        .split(documents_area);

    for (i, doc) in visible_docs.iter().enumerate() {
        let mut lines: Vec<Line> = Vec::new();

        for (field_index, (key, value)) in doc.iter().enumerate() {
            let value_str = format_bson_value(value);
            let real_index = state.doc_scroll_offset + i;
            let is_selected = state.focus == FocusArea::Documents
                && state.selected_doc_index == state.doc_scroll_offset + i
                && state.selected_field_index == field_index;
            let is_expanded = state.expanded_field == Some((real_index, field_index));

            push_field_lines(
                &mut lines,
                key,
                &value_str,
                is_selected,
                is_expanded,
                &theme,
            );
        }

        let text = Text::from(lines);
        let real_index = state.doc_scroll_offset + i;

        let block = Block::default()
            .title(format!("Document #{}", real_index + 1))
            .borders(Borders::ALL)
            .border_style(
                if state.focus == FocusArea::Documents && state.selected_doc_index == real_index {
                    Style::default().fg(theme.accent).bg(theme.secondary)
                } else {
                    Style::default()
                },
            );

        let paragraph = Paragraph::new(text)
            .block(block)
            .scroll((state.document_line_scroll, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[i]);
    }
}
