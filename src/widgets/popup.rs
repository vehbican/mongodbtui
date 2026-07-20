use crate::app::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_popup(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = state.theme.palette();
    if let Some(message) = &state.popup_message {
        let popup_area = Rect {
            x: area.width / 3,
            y: area.height / 3 - 5,
            width: area.width / 2,
            height: 3,
        };

        let block = Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.error));

        let paragraph = Paragraph::new(Text::from(Span::raw(message)))
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.foreground).bg(theme.background));

        f.render_widget(paragraph, popup_area);
    }
}
pub fn render_popup_success(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = state.theme.palette();
    if let Some(message) = &state.popup_message_success {
        let popup_area = Rect {
            x: area.width / 3,
            y: area.height / 3 - 5,
            width: area.width / 2,
            height: 3,
        };

        let block = Block::default()
            .title("Success")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.success));

        let paragraph = Paragraph::new(Text::from(Span::raw(message)))
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.foreground).bg(theme.background));

        f.render_widget(paragraph, popup_area);
    }
}
