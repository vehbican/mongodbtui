use crate::app::AppState;
use crate::keybindings::handle_by_mode;
use crate::tui::fpicker_events;
use crossterm::event::KeyEvent;

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
