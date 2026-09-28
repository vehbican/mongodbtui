use crate::{
    app::{AppMode, AppState, FocusArea, SelectableItem},
    keybindings::normal::handle_normal,
    theme::ThemeName,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct CommandPalette {
    query: String,
    selected: usize,
}

#[derive(Clone, Copy)]
enum Target {
    Connections,
    Documents,
}

#[derive(Clone, Copy)]
enum Action {
    Key(KeyCode, Option<Target>),
    Delete(Target),
    Focus(Target),
    Theme(ThemeName),
    ResetQuery,
    Help,
}

struct Command {
    label: String,
    shortcut: &'static str,
    action: Action,
}

fn commands(state: &AppState) -> Vec<Command> {
    use Action::*;
    use KeyCode::Char;
    use Target::*;
    let mut result = Vec::new();
    let mut add = |label: &str, shortcut, action| {
        result.push(Command {
            label: label.into(),
            shortcut,
            action,
        });
    };
    add("Add connection", "o", Key(Char('o'), None));
    add("Search collections", "/", Key(Char('/'), Some(Connections)));
    add("Focus connections", "Ctrl+h", Focus(Connections));
    add("Focus documents", "Ctrl+l", Focus(Documents));
    add("Run script from file…", "f", Key(Char('f'), None));
    add("Show keyboard shortcuts", "?", Help);
    for theme in ThemeName::ALL {
        add(
            &format!("Change theme: {}", theme.as_str()),
            "",
            Theme(theme),
        );
    }
    if let Some(item) = state.tree_items.get(state.selected_index) {
        match item {
            SelectableItem::Uri { name, .. } => {
                add(
                    &format!("Connect: {name}"),
                    "Enter",
                    Key(KeyCode::Enter, Some(Connections)),
                );
                add(
                    &format!("Edit connection: {name}"),
                    "e",
                    Key(Char('e'), Some(Connections)),
                );
                add(
                    &format!("Import database: {name}"),
                    "I",
                    Key(Char('I'), Some(Connections)),
                );
            }
            SelectableItem::Database { uri, name } if state.connected_uri.as_ref() == Some(uri) => {
                add(
                    &format!("Expand/collapse database: {name}"),
                    "Enter",
                    Key(KeyCode::Enter, Some(Connections)),
                );
                add(
                    &format!("Import collection into: {name}"),
                    "i",
                    Key(Char('i'), Some(Connections)),
                );
                add(
                    &format!("Export database: {name}"),
                    "x",
                    Key(Char('x'), Some(Connections)),
                );
                add(
                    &format!("Delete database: {name}"),
                    "dd",
                    Delete(Connections),
                );
            }
            SelectableItem::Collection { uri, db, name }
                if state.connected_uri.as_ref() == Some(uri) =>
            {
                add(
                    &format!("Open collection: {db}.{name}"),
                    "Enter",
                    Key(KeyCode::Enter, Some(Connections)),
                );
                add(
                    &format!("Rename collection: {db}.{name}"),
                    "e",
                    Key(Char('e'), Some(Connections)),
                );
                add(
                    &format!("Export collection: {db}.{name}"),
                    "x",
                    Key(Char('x'), Some(Connections)),
                );
                add(
                    &format!("Delete collection: {db}.{name}"),
                    "dd",
                    Delete(Connections),
                );
            }
            _ => {}
        }
    }
    if let Some((uri, db, collection)) = &state.selected_collection {
        if state.mongo_client.is_some() && state.connected_uri.as_ref() == Some(uri) {
            let target = format!("{db}.{collection}");
            add(
                &format!("Edit filter: {target}"),
                "/",
                Key(Char('/'), Some(Documents)),
            );
            add(
                &format!("Edit sort: {target}"),
                "s",
                Key(Char('s'), Some(Documents)),
            );
            add(&format!("Reset filter and sort: {target}"), "", ResetQuery);
            add(
                &format!("Bulk update filtered documents: {target}"),
                "U",
                Key(Char('U'), Some(Documents)),
            );
            add(
                &format!("Delete filtered documents: {target}"),
                "X",
                Key(Char('X'), Some(Documents)),
            );
            if let Some(document) = state.current_documents.get(state.selected_doc_index) {
                add(
                    "Edit selected document",
                    "e",
                    Key(Char('e'), Some(Documents)),
                );
                if document.get_object_id("_id").is_ok() {
                    add("Delete selected document", "dd", Delete(Documents));
                }
                if let Some((field, _)) = document.iter().nth(state.selected_field_index) {
                    add(
                        &format!("Copy selected field as filter: {field}"),
                        "y",
                        Key(Char('y'), Some(Documents)),
                    );
                    if field != "_id" && document.get_object_id("_id").is_ok() {
                        add(
                            &format!("Delete selected field: {field}"),
                            "D",
                            Key(Char('D'), Some(Documents)),
                        );
                    }
                }
            }
        }
    }
    add("Quit application", "q", Key(Char('q'), None));
    result
}

fn matches_query(label: &str, query: &str) -> bool {
    let label = label.to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| label.contains(word))
}

fn filtered_commands(state: &AppState, query: &str) -> Vec<Command> {
    commands(state)
        .into_iter()
        .filter(|c| matches_query(&c.label, query))
        .collect()
}

pub fn can_open(state: &AppState) -> bool {
    state.mode == AppMode::Normal
        && state.file_picker.is_none()
        && state.pending_deletion.is_none()
        && state.pending_bulk_update.is_none()
        && state.pending_bulk_deletion.is_none()
}

pub fn paste(state: &mut AppState, text: &str) {
    if let Some(palette) = &mut state.command_palette {
        palette
            .query
            .extend(text.chars().filter(|c| !c.is_control()));
        palette.selected = 0;
    }
}

fn set_focus(state: &mut AppState, target: Target) {
    state.focus = match target {
        Target::Connections => FocusArea::Connections,
        Target::Documents => FocusArea::Documents,
    };
}

pub async fn handle_key(key: KeyEvent, state: &mut AppState) -> bool {
    let Some(mut palette) = state.command_palette.take() else {
        return false;
    };
    let commands = filtered_commands(state, &palette.query);
    match key.code {
        KeyCode::Esc => return false,
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,
        KeyCode::Down => {
            palette.selected = (palette.selected + 1).min(commands.len().saturating_sub(1))
        }
        KeyCode::Up => palette.selected = palette.selected.saturating_sub(1),
        KeyCode::Backspace => {
            if let Some((index, _)) = palette.query.grapheme_indices(true).next_back() {
                palette.query.truncate(index);
            }
            palette.selected = 0;
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            palette.query.push(c);
            palette.selected = 0;
        }
        KeyCode::Enter => {
            if let Some(command) = commands.get(palette.selected) {
                state.last_key = None;
                state.popup_message = None;
                state.popup_message_success = None;
                match command.action {
                    Action::Key(code, target) => {
                        if let Some(target) = target {
                            set_focus(state, target);
                        }
                        return handle_normal(KeyEvent::new(code, KeyModifiers::NONE), state).await;
                    }
                    Action::Delete(target) => {
                        set_focus(state, target);
                        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
                        handle_normal(key, state).await;
                        return handle_normal(key, state).await;
                    }
                    Action::Focus(target) => set_focus(state, target),
                    Action::Theme(theme) => {
                        state.theme = theme;
                        if let Err(error) = crate::utils::save_theme(theme) {
                            state.popup_message = Some(format!("Could not save theme: {error}"));
                        }
                    }
                    Action::ResetQuery => {
                        state.filter_text = "{}".into();
                        state.sort_text = "{}".into();
                        state.reload_documents_for_selected_collection().await;
                    }
                    Action::Help => {
                        state.show_help = true;
                        state.help_scroll = 0;
                    }
                }
                return false;
            }
        }
        _ => {}
    }
    state.command_palette = Some(palette);
    false
}

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let Some(palette) = &state.command_palette else {
        return;
    };
    let screen = frame.area();
    let width = screen.width.saturating_sub(4).min(90);
    let height = screen.height.saturating_sub(2).min(20);
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    let theme = state.theme.palette();
    let block = Block::default()
        .title(" Command palette ")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.background).fg(theme.foreground))
        .border_style(Style::default().fg(theme.primary));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);
    let input_width = rows[0].width.saturating_sub(3) as usize;
    let mut visible = String::new();
    for grapheme in palette.query.graphemes(true).rev() {
        if visible.width() + grapheme.width() > input_width {
            break;
        }
        visible.insert_str(0, grapheme);
    }
    frame.render_widget(Paragraph::new(format!("> {visible}")), rows[0]);
    if rows[0].width >= 3 && rows[0].height > 0 {
        frame.set_cursor_position((rows[0].x + 2 + visible.width() as u16, rows[0].y));
    }
    let commands = filtered_commands(state, &palette.query);
    if commands.is_empty() {
        frame.render_widget(
            Paragraph::new("No matching commands").style(Style::default().fg(theme.muted)),
            rows[1],
        );
    } else {
        let items: Vec<_> = commands
            .iter()
            .map(|command| {
                ListItem::new(Line::from(vec![
                    Span::raw(&command.label),
                    Span::styled(
                        format!("  {}", command.shortcut),
                        Style::default().fg(theme.muted),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).highlight_symbol("› ").highlight_style(
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        );
        let mut selection =
            ListState::default().with_selected(Some(palette.selected.min(commands.len() - 1)));
        frame.render_stateful_widget(list, rows[1], &mut selection);
    }
    frame.render_widget(
        Paragraph::new("↑↓ select  Enter run  Esc close").style(Style::default().fg(theme.muted)),
        rows[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::events::{handle_key_event, handle_paste_event};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[tokio::test]
    async fn palette_captures_input_and_dispatches_selected_action() {
        let mut state = AppState::default();
        let open = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        handle_key_event(open, &mut state).await;
        assert!(state.command_palette.is_some());
        assert!(!handle_key_event(key(KeyCode::Char('q')), &mut state).await);
        assert!(state.command_palette.is_some());
        handle_key_event(key(KeyCode::Backspace), &mut state).await;
        handle_paste_event("FOCUS doc\n".into(), &mut state);
        handle_key_event(key(KeyCode::Enter), &mut state).await;
        assert!(state.focus == FocusArea::Documents);
        assert!(state.command_palette.is_none());
        assert!(state.input_text.is_empty());
    }

    #[tokio::test]
    async fn deletion_uses_confirmation_and_blocks_reopening() {
        let mut state = AppState::default();
        state.connected_uri = Some("local".into());
        state.tree_items.push(SelectableItem::Database {
            uri: "local".into(),
            name: "shop".into(),
        });
        state.command_palette = Some(CommandPalette {
            query: "delete database".into(),
            selected: 0,
        });
        handle_key_event(key(KeyCode::Enter), &mut state).await;
        assert!(state.pending_deletion.is_some());
        assert!(!can_open(&state));
        handle_key_event(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &mut state,
        )
        .await;
        assert!(state.command_palette.is_none());
        assert!(state.pending_deletion.is_some());
        handle_key_event(key(KeyCode::Esc), &mut state).await;
        assert!(state.pending_deletion.is_none());
    }

    #[tokio::test]
    async fn empty_results_and_unicode_input_are_safe() {
        let mut state = AppState::default();
        state.command_palette = Some(Default::default());
        paste(&mut state, "不存在👩‍💻");
        handle_key_event(key(KeyCode::Backspace), &mut state).await;
        assert_eq!(state.command_palette.as_ref().unwrap().query, "不存在");
        handle_key_event(key(KeyCode::Down), &mut state).await;
        handle_key_event(key(KeyCode::Enter), &mut state).await;
        assert!(state.command_palette.is_some());
        handle_key_event(key(KeyCode::Esc), &mut state).await;
        assert!(state.command_palette.is_none());
    }

    #[test]
    fn contextual_search_and_small_terminal_rendering() {
        let mut state = AppState::default();
        assert!(filtered_commands(&state, "delete").is_empty());
        assert!(matches_query("Export collection: shop.users", "EXP col"));
        state.command_palette = Some(CommandPalette {
            query: "theme".into(),
            selected: 4,
        });
        for (width, height) in [(80, 24), (20, 8), (3, 2), (1, 1)] {
            let backend = ratatui::backend::TestBackend::new(width, height);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal.draw(|frame| render(frame, &state)).unwrap();
        }
        state.mode = AppMode::Insert;
        assert!(!can_open(&state));
    }
}
