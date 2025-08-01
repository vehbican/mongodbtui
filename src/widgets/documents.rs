use crate::app::{AppState, FocusArea};
use bson::Bson;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render_documents(f: &mut Frame, area: Rect, state: &AppState) {
    let documents = &state.current_documents;

    if documents.is_empty() {
        let block = Block::default()
            .title("Documents")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let empty = Paragraph::new("No documents to display")
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(empty, area);
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
        .split(area);

    for (i, doc) in visible_docs.iter().enumerate() {
        let mut lines: Vec<Line> = Vec::new();

        for (field_index, (key, value)) in doc.iter().enumerate() {
            let value_str = match value {
                Bson::ObjectId(oid) => format!("ObjectId(\"{}\")", oid),
                Bson::DateTime(dt) => dt.to_string(),
                Bson::Document(inner) => match serde_json::to_string_pretty(inner) {
                    Ok(s) => s,
                    Err(_) => "<invalid document>".to_string(),
                },
                _ => format!("{}", value),
            };

            let formatted = format!("{:<15}: {}", key, value_str);

            if state.focus == FocusArea::Documents
                && state.selected_doc_index == state.doc_scroll_offset + i
                && state.selected_field_index == field_index
            {
                lines.push(Line::from(vec![Span::styled(
                    formatted,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]));
            } else {
                lines.push(Line::from(formatted));
            }
        }

        let text = Text::from(lines);
        let real_index = state.doc_scroll_offset + i;

        let block = Block::default()
            .title(format!("Document #{}", real_index + 1))
            .borders(Borders::ALL)
            .border_style(
                if state.focus == FocusArea::Documents && state.selected_doc_index == real_index {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default()
                },
            );

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[i]);
    }
}
