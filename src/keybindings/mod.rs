pub mod editor;
pub mod insert;
pub mod normal;

use crate::app::AppState;
use crossterm::event::KeyEvent;

pub async fn handle_by_mode(key: KeyEvent, state: &mut AppState) -> bool {
    use crate::app::AppMode::*;
    match state.mode {
        Normal => normal::handle_normal(key, state).await,
        Insert => insert::handle_insert(key, state).await,
    }
}
