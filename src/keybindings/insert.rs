use crate::app::SelectableItem;
use crate::app::{ActiveInputField, AppMode, AppState, InputContext};
use crate::tui::events::{clamp_cursor, goto_collection, is_braced_object};
use crate::utils::read_clipboard_string;
use crate::utils::{load_connections, parse_connection_input, save_connection};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_paste_text(mut text: String, state: &mut AppState) {
    if text.ends_with('\n') {
        text.pop();
    }

    let mut paste_at = |target: &mut String| {
        let pos = state.cursor_position.min(target.chars().count());
        let byte_pos = target
            .char_indices()
            .nth(pos)
            .map(|(i, _)| i)
            .unwrap_or(target.len());
        target.insert_str(byte_pos, &text);
        state.cursor_position = pos + text.chars().count();
    };

    if state.input_context != InputContext::None {
        paste_at(&mut state.input_text);
    } else {
        match state.active_input {
            Some(ActiveInputField::Filter) => paste_at(&mut state.filter_text),
            Some(ActiveInputField::Sort) => paste_at(&mut state.sort_text),
            _ => {}
        }
    }
}

pub async fn handle_insert(key: KeyEvent, state: &mut AppState) -> bool {
    match key.code {
        KeyCode::Enter => {
            let keep_active_input =
                state.input_context == InputContext::None && state.active_input.is_some();

            match state.input_context {
                InputContext::Uri => {
                    if let Some((uri, name)) = parse_connection_input(&state.input_text) {
                        match save_connection(&uri, &name) {
                            Ok(()) => {
                                state.connections = load_connections().unwrap_or_default();
                                state.rebuild_tree_items();
                                state.popup_message = None;
                                state.popup_message_success = None;
                            }
                            Err(e) => {
                                state.popup_message = Some(e.to_string());
                                return false;
                            }
                        }
                    } else {
                        state.popup_message =
                            Some("Invalid input. Use format: URI;NAME".to_string());
                        return false;
                    }
                }

                InputContext::ConnectionName => {
                    match crate::utils::update_connection(&state.input_text) {
                        Ok(_) => {
                            state.connections =
                                crate::utils::load_connections().unwrap_or_default();
                            state.rebuild_tree_items();
                        }
                        Err(e) => {
                            state.popup_message = Some(e.to_string());
                            return false;
                        }
                    }
                }
                InputContext::CollectionName => {
                    if let Some((uri, db, old_name)) = &state.selected_collection {
                        let new_name = state.input_text.trim();
                        if new_name.is_empty() || new_name == old_name {
                            state.popup_message =
                                Some("❗ New collection name is empty or unchanged.".to_string());
                            return false;
                        }

                        match &state.mongo_client {
                            Some(client) => {
                                match crate::db::client::rename_collection(
                                    client, db, old_name, new_name,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        state.collection_to_load = Some((uri.clone(), db.clone()));
                                    }
                                    Err(e) => {
                                        state.popup_message =
                                            Some(format!("❌ Rename failed: {}", e));
                                    }
                                }
                            }

                            None => {
                                state.popup_message =
                                    Some("No active MongoDB connection".to_string());
                                return false;
                            }
                        }
                    }
                }
                InputContext::SearchCollections => {
                    let q = state.input_text.trim().to_lowercase();
                    state.collection_search_hits.clear();
                    state.collection_search_idx = 0;

                    if !q.is_empty() {
                        for item in &state.tree_items {
                            if let SelectableItem::Collection { uri, db, name } = item {
                                let hay = format!("{}/{}/{}", uri, db, name).to_lowercase();
                                if hay.contains(&q) {
                                    state.collection_search_hits.push((
                                        uri.clone(),
                                        db.clone(),
                                        name.clone(),
                                    ));
                                }
                            }
                        }
                    }

                    state.mode = AppMode::Normal;
                    state.input_context = InputContext::None;
                    state.input_text.clear();

                    if let Some((uri, db, name)) = state.collection_search_hits.first().cloned() {
                        goto_collection(state, &uri, &db, &name);
                    } else {
                        state.popup_message = Some(format!("🔎 No match: \"{}\"", q));
                    }
                }

                InputContext::None => {}
            }
            state.mode = AppMode::Normal;
            state.input_context = InputContext::None;
            state.input_text.clear();
            if !keep_active_input {
                state.active_input = None;
            }
        }

        KeyCode::Esc => {
            if state.popup_message.is_some() {
                state.popup_message = None;
                state.popup_message_success = None;
            } else {
                let keep_active_input =
                    state.input_context == InputContext::None && state.active_input.is_some();

                state.mode = AppMode::Normal;
                state.input_context = InputContext::None;
                if !keep_active_input {
                    state.active_input = None;
                }
            }
        }

        KeyCode::Char('v')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if let Ok(clip) = read_clipboard_string() {
                handle_paste_text(clip, state);
            } else {
                state.popup_message = Some("❌ Could not read clipboard (Ctrl+V)".to_string());
            }
        }
        KeyCode::Char(c) => {
            if state.input_context != InputContext::None {
                let mut chars: Vec<char> = state.input_text.chars().collect();
                chars.insert(state.cursor_position, c);
                state.input_text = chars.iter().collect();
                state.cursor_position += 1;
            } else {
                match state.active_input {
                    Some(ActiveInputField::Filter) => {
                        let mut chars: Vec<char> = state.filter_text.chars().collect();
                        chars.insert(state.cursor_position, c);
                        state.filter_text = chars.iter().collect();
                        state.cursor_position += 1;
                    }
                    Some(ActiveInputField::Sort) => {
                        let mut chars: Vec<char> = state.sort_text.chars().collect();
                        chars.insert(state.cursor_position, c);
                        state.sort_text = chars.iter().collect();
                        state.cursor_position += 1;
                    }
                    _ => {}
                }
            }
        }

        KeyCode::Backspace => {
            if state.input_context != InputContext::None && state.cursor_position > 0 {
                let mut chars: Vec<char> = state.input_text.chars().collect();
                chars.remove(state.cursor_position - 1);
                state.input_text = chars.iter().collect();
                state.cursor_position -= 1;
            } else {
                match state.active_input {
                    Some(ActiveInputField::Filter) => {
                        if state.cursor_position > 0 {
                            if is_braced_object(&state.filter_text) && state.cursor_position == 1 {
                            } else {
                                let mut chars: Vec<char> = state.filter_text.chars().collect();
                                chars.remove(state.cursor_position - 1);
                                state.filter_text = chars.into_iter().collect();
                                state.cursor_position -= 1;
                                state.cursor_position =
                                    clamp_cursor(state.cursor_position, &state.filter_text);
                            }
                        }
                    }
                    Some(ActiveInputField::Sort) => {
                        if state.cursor_position > 0 {
                            if is_braced_object(&state.sort_text) && state.cursor_position == 1 {
                            } else {
                                let mut chars: Vec<char> = state.sort_text.chars().collect();
                                chars.remove(state.cursor_position - 1);
                                state.sort_text = chars.into_iter().collect();
                                state.cursor_position -= 1;
                                state.cursor_position =
                                    clamp_cursor(state.cursor_position, &state.sort_text);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Left => {
            if state.cursor_position > 0 {
                state.cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            let len = if state.input_context != InputContext::None {
                state.input_text.chars().count()
            } else {
                match state.active_input {
                    Some(ActiveInputField::Filter) => state.filter_text.chars().count(),
                    Some(ActiveInputField::Sort) => state.sort_text.chars().count(),
                    _ => 0,
                }
            };
            if state.cursor_position < len {
                state.cursor_position += 1;
            }
        }

        _ => {}
    }
    false
}
