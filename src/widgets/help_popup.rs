use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub const HELP_TEXT: &str = r#"
Global:
?           Toggle help popup
t           Cycle theme (system, emerald, ocean, rose, monochrome)
q           Quit the application
Esc         Dismiss popup / clear search hits

Focus Navigation:
Ctrl+l      Focus → Documents
Ctrl+h      Focus → Connections

List Navigation:
j / ↓       Move down
k / ↑       Move up
Enter       Connect / expand database / load collection

Connections & Collections:
o           Add new MongoDB connection
/           Search collections
n / N       Next / previous collection search match
e           Edit selected connection or collection name
x           Export selected collection or database
i           Import collection into selected database
I           Import database into selected connection
f           Run shell script from file picker
d+d         Delete selected collection or database

Documents:
/           Edit filter command
s           Edit sort command
Enter       Expand/collapse selected field
n / N       Next / previous field in selected document
e           Edit selected document in external editor
y           Copy selected field as filter fragment
d+d         Delete selected document
D           Delete selected field (except _id)

Insert Mode:
Enter       Submit input / apply filter or sort
Esc         Cancel editing
← / →       Move cursor
Backspace   Delete character
Ctrl+V      Paste clipboard
Ctrl+Shift+V Paste from terminal

File Picker (import/export/script):
j / k       Navigate entries
Space       Select/Deselect file
Enter       Enter directory
c           Confirm action (import/run)
Esc         Exit file picker


"#;

pub fn draw_help_popup(f: &mut Frame<'_>, area: Rect, scroll: usize, theme: &Theme) {
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
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if trimmed.is_empty() {
                Line::from("")
            } else if let Some((key, desc)) = line.split_once("  ") {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(theme.muted)),
                    Span::raw("  "),
                    Span::styled(desc.trim_start(), Style::default().fg(theme.secondary)),
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
                .style(Style::default().fg(theme.primary))
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
