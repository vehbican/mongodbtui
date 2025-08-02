use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub const HELP_TEXT: &str = r#"
Global:
?           Toggle help popup
q           Quit the application
Esc         Dismiss popup / Exit insert mode

Focus Navigation:
Ctrl+l      Focus → Filter/Sort
Ctrl+j      Focus → Documents
Ctrl+k      Focus → Filter/Sort
Ctrl+h      Focus → Connections

List Navigation:
j / ↓       Move down
k / ↑       Move up
Enter       Expand item / Load collection / Confirm

Connections & Collections:
o           Add new MongoDB connection
e           Edit selected URI or collection name
x           Export selected collection or database
d+d       Delete hovered item
              • In Filter: deletes matched documents
              • In Documents: deletes selected document
              • In Connections: deletes collection or database

Filter & Sort:
a           Edit filter or sort input
Tab         Toggle between filter and sort input
Enter       Apply filter & sort

Documents:
n / N       Navigate fields in document
e           Edit selected field
D           Delete selected field (except _id)

Insert Mode:
Enter       Submit input
Esc         Cancel editing
← / →       Move cursor
Backspace   Delete character

File Picker (import/export/script):
i           Import a collection (.json)
I           Import a database (from folder)
f           Run a shell script (.sh)
j / k       Navigate entries
Space       Select/Deselect file
Enter       Enter directory
c           Confirm action (import/run)
Esc         Exit file picker

Editor Mode (Full-screen document editor):
Esc     Exit editor without saving
Enter       Parse and save document (must include valid _id)
← / →       Move cursor left/right
↑ / ↓       Move cursor up/down
Backspace   Delete character


"#;

pub fn draw_help_popup(f: &mut Frame<'_>, area: Rect, scroll: usize) {
    let popup_area = centered_rect(70, 70, area);
    let max_height = popup_area.height.saturating_sub(2) as usize;

    let lines: Vec<Line> = HELP_TEXT
        .lines()
        .map(|line| {
            let trimmed = line.trim();

            if trimmed.ends_with(':') {
                Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if trimmed.is_empty() {
                Line::from("")
            } else if let Some((key, desc)) = line.split_once("  ") {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(Color::Gray)),
                    Span::raw("  "),
                    Span::styled(desc.trim_start(), Style::default().fg(Color::Yellow)),
                ])
            } else {
                Line::from(Span::raw(line))
            }
        })
        .collect();

    let visible_lines = lines
        .iter()
        .skip(scroll)
        .take(max_height)
        .cloned()
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .title(" MongoDB TUI Help ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Green))
                .border_type(ratatui::widgets::BorderType::Rounded),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
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
        .split(popup_layout[1])[1]
}
