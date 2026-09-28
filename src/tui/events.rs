use crate::app::{AppMode, AppState};
use crate::app::{FocusArea, SelectableItem};
use crate::keybindings::{handle_by_mode, insert};
use crate::tui::fpicker_events;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn is_braced_object(s: &str) -> bool {
    let t = s.trim();
    t.starts_with('{') && t.ends_with('}')
}
pub fn inner_end_pos(s: &str) -> usize {
    let len = s.chars().count();
    if is_braced_object(s) && len >= 2 {
        len - 1
    } else {
        len
    }
}
pub fn clamp_cursor(pos: usize, s: &str) -> usize {
    let len = s.chars().count();
    pos.min(len)
}
pub async fn handle_key_event(key: KeyEvent, state: &mut AppState) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }
    if state.command_palette.is_some() {
        return crate::command_palette::handle_key(key, state).await;
    }
    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if crate::command_palette::can_open(state) {
            state.command_palette = Some(Default::default());
            state.last_key = None;
            state.show_help = false;
        }
        return false;
    }
    if state.file_picker.is_some() {
        let should_close = fpicker_events::handle_filepicker_key(key, state).await;
        if should_close {
            state.file_picker = None;
        }
        return false;
    }

    handle_by_mode(key, state).await
}

pub fn handle_paste_event(text: String, state: &mut AppState) {
    if state.command_palette.is_some() {
        crate::command_palette::paste(state, &text);
        return;
    }
    if state.file_picker.is_some() || state.mode != AppMode::Insert {
        return;
    }

    insert::handle_paste_text(text, state);
}

pub fn goto_collection(state: &mut AppState, uri: &str, db: &str, name: &str) {
    state.expanded_uris.insert(uri.to_string());
    state.expanded_dbs.insert((uri.to_string(), db.to_string()));

    // state.rebuild_tree_items();

    if let Some(idx) = state.tree_items.iter().position(|it| {
        matches!(it, SelectableItem::Collection { uri: u, db: d, name: n }
            if u == uri && d == db && n == name)
    }) {
        state.selected_index = idx;
        state.focus = FocusArea::Connections;
    }
}
