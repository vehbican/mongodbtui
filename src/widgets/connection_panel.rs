use crate::app::{AppState, FocusArea, SelectableItem};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub fn render_connections(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = state.theme.palette();
    let items: Vec<ListItem> = state
        .tree_items
        .iter()
        .map(|item| match item {
            SelectableItem::Uri {
                name, connected, ..
            } => {
                let content = if *connected {
                    Line::from(vec![
                        Span::styled("✔ ", Style::default().fg(theme.success)),
                        Span::styled(name, Style::default().fg(theme.success)),
                    ])
                } else {
                    Line::from(Span::raw(name))
                };
                ListItem::new(content)
            }
            SelectableItem::Database { name, .. } => {
                ListItem::new(Line::from(format!("  └ {}", name)))
            }
            SelectableItem::Collection { name, .. } => {
                ListItem::new(Line::from(format!("      • {}", name)))
            }
        })
        .collect();
    let mut list_state = ListState::default();
    if !items.is_empty() {
        list_state.select(Some(state.selected_index));
    }

    let highlight_style = if state.focus == FocusArea::Connections {
        Style::default().bg(theme.primary).fg(theme.accent)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Connections")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .highlight_style(highlight_style);

    f.render_stateful_widget(list, area, &mut list_state);
}
