use crate::app::AppState;
use crate::app::{FocusArea, SelectableItem};
use crate::keybindings::handle_by_mode;
use crate::tui::fpicker_events;
use crossterm::event::KeyEvent;

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
    if state.file_picker.is_some() {
        let should_close = fpicker_events::handle_filepicker_key(key, state).await;
        if should_close {
            state.file_picker = None;
        }
        return false;
    }

    handle_by_mode(key, state).await
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
