use crate::app::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_popup(f: &mut Frame, area: Rect, state: &AppState) {
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
            .border_style(Style::default().fg(Color::Red));

        let paragraph = Paragraph::new(Text::from(Span::raw(message)))
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(paragraph, popup_area);
    }
}
pub fn render_popup_success(f: &mut Frame, area: Rect, state: &AppState) {
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
            .border_style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(Text::from(Span::raw(message)))
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(paragraph, popup_area);
    }
}
