use crate::app::{AppState, SelectableItem};
use crate::db::import_export::import_database;
use crate::tui::filepicker::{FileEntry, FilePickerMode};
use crossterm::event::{KeyCode, KeyEvent};

pub async fn handle_filepicker_key(event: KeyEvent, state: &mut AppState) -> bool {
    match event.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(picker) = &mut state.file_picker {
                picker.next();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(picker) = &mut state.file_picker {
                picker.previous();
            }
        }
        KeyCode::Char(' ') => {
            if let Some(picker) = &mut state.file_picker {
                picker.toggle_selection();
            }
        }
        KeyCode::Enter => {
            if let Some(picker) = &mut state.file_picker {
                if let Some(entry) = picker.selected_entry() {
                    if let Some(path) = match entry {
                        crate::tui::filepicker::FileEntry::Real(e) => Some(e.path()),
                        crate::tui::filepicker::FileEntry::Parent(p) => Some(p.clone()),
                    } {
                        if path.is_dir() {
                            picker.enter_directory(&path);
                        }
                    }
                }
            }
        }

        KeyCode::Char('c') => {
            let Some(picker) = &state.file_picker else {
                return false;
            };

            let Some(client) = &state.mongo_client else {
                state.popup_message = Some("❗ Not connected to MongoDB.".to_string());
                return false;
            };

            match picker.mode {
                FilePickerMode::ImportCollection => {
                    let Some(SelectableItem::Database { uri, name: db_name }) =
                        state.tree_items.get(state.selected_index)
                    else {
                        state.popup_message = Some("❗ No database selected.".to_string());
                        return false;
                    };

                    let (success, failed) = picker.perform_import(client, db_name, uri).await;
                    state.popup_message_success = Some(format!(
                        "📥 Imported: {} ✅ | ❌ Failed: {}",
                        success, failed
                    ));

                    state.collection_to_load = Some((uri.clone(), db_name.clone()));
                }

                FilePickerMode::ImportDatabase => {
                    let Some(FileEntry::Real(entry)) = picker.selected_entry() else {
                        state.popup_message = Some("❗ Please select a folder.".to_string());
                        return false;
                    };

                    let folder_path = entry.path();

                    if !folder_path.is_dir() {
                        state.popup_message = Some("❗ Please select a folder.".to_string());
                        return false;
                    }

                    let Some(SelectableItem::Uri { uri, .. }) =
                        state.tree_items.get(state.selected_index)
                    else {
                        state.popup_message = Some("❗ No connection selected.".to_string());
                        return false;
                    };

                    let db_name = folder_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("imported_db");

                    match import_database(client, db_name, folder_path.to_str().unwrap()).await {
                        Ok(_) => {
                            state.popup_message_success =
                                Some(format!("✅ Imported folder as database '{}'", db_name));
                            state.collection_to_load = Some((uri.clone(), db_name.to_string()));
                        }
                        Err(e) => {
                            state.popup_message = Some(format!("❌ Import failed: {}", e));
                        }
                    }
                }
                FilePickerMode::RunScript => {
                    let Some(FileEntry::Real(entry)) = picker.selected_entry() else {
                        state.popup_message =
                            Some("❗ Please select a .sh script file.".to_string());
                        return false;
                    };

                    let path = entry.path();

                    if path.extension().and_then(|ext| ext.to_str()) != Some("sh") {
                        state.popup_message = Some("❗ Only .sh files are allowed.".to_string());
                        return false;
                    }

                    match std::process::Command::new("sh").arg(path).output() {
                        Ok(output) => {
                            if output.status.success() {
                                state.popup_message_success =
                                    Some("✅ Script executed successfully.".to_string());
                            } else {
                                let err = String::from_utf8_lossy(&output.stderr);
                                state.popup_message = Some(format!("❌ Script failed: {}", err));
                            }
                        }
                        Err(e) => {
                            state.popup_message = Some(format!("❌ Failed to run script: {}", e));
                        }
                    }

                    state.file_picker = None;
                }
            }

            state.file_picker = None;
        }

        KeyCode::Esc => {
            state.file_picker = None;
            return true;
        }
        _ => {}
    }

    false
}
