use crate::app::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Span, Text},
    widgets::{Block, Borders, Gauge, Paragraph},
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

pub fn render_export_progress(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(progress) = &state.export_progress else {
        return;
    };
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 2,
        width: area.width / 2,
        height: 4,
    };
    let ratio = if progress.total == 0 {
        0.0
    } else {
        (progress.completed as f64 / progress.total as f64).min(1.0)
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(format!("Exporting {}", progress.label))
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(state.theme.palette().primary))
        .label(format!(
            "Collection {}/{}  •  {}/{} documents",
            progress.collection_completed,
            progress.collection_total,
            progress.completed,
            progress.total,
        ))
        .ratio(ratio);
    f.render_widget(gauge, popup_area);
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
