use crate::tui::filepicker::{FileEntry, FilePickerState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

pub fn render_file_picker(f: &mut Frame, area: Rect, picker: &FilePickerState) {
    let outer_block = Block::default()
        .title("📁 Import File Picker")
        .borders(Borders::ALL);

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let items: Vec<ListItem> = picker
        .entries
        .iter()
        .map(|entry| {
            let name = match entry {
                FileEntry::Real(e) => e.file_name().to_string_lossy().to_string(),
                FileEntry::Parent(_) => "..".to_string(),
            };

            let path = match entry {
                FileEntry::Real(e) => Some(e.path()),
                FileEntry::Parent(p) => Some(p.clone()),
            };

            let prefix = match entry {
                FileEntry::Real(e) => {
                    if e.path().is_dir() {
                        "📂 "
                    } else {
                        "📄 "
                    }
                }
                FileEntry::Parent(_) => "⬆️  ",
            };

            let style = if let Some(p) = &path {
                if picker.selected_files.contains(p) {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, name),
                style,
            )))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(picker.selected_index));

    let list = List::new(items)
        .block(
            Block::default()
                .title(picker.current_path.display().to_string())
                .borders(Borders::ALL),
        )
        .highlight_symbol("➤ ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, inner_area, &mut list_state);
}
