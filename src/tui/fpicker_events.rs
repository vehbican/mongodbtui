use crate::app::{AppState, SelectableItem};
use crate::db::import_export::{import_collection_with_progress, import_database_with_progress};
use crate::tui::filepicker::{FileEntry, FilePickerMode};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

fn start_collection_import(
    state: &mut AppState,
    client: mongodb::Client,
    db_name: String,
    files: Vec<(String, String)>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    state.export_progress = Some(crate::db::import_export::ExportProgress {
        label: "Import".to_string(),
        completed: 0,
        total: 0,
        collection_completed: 0,
        collection_total: files.len(),
    });
    state.export_progress_rx = Some(receiver);
    state.export_task = Some(tokio::spawn(async move {
        let total = files.len();
        for (index, (collection, path)) in files.into_iter().enumerate() {
            import_collection_with_progress(
                &client,
                &db_name,
                &collection,
                &path,
                Some(&sender),
                index + 1,
                total,
            )
            .await
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }));
}

fn start_database_import(
    state: &mut AppState,
    client: mongodb::Client,
    db_name: String,
    path: String,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    state.export_progress = Some(crate::db::import_export::ExportProgress {
        label: "Import".to_string(),
        completed: 0,
        total: 0,
        collection_completed: 0,
        collection_total: 0,
    });
    state.export_progress_rx = Some(receiver);
    state.export_task = Some(tokio::spawn(async move {
        import_database_with_progress(&client, &db_name, &path, Some(sender))
            .await
            .map_err(|error| error.to_string())
    }));
}

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

                    let files = picker
                        .selected_files
                        .iter()
                        .filter_map(|path| {
                            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                                return None;
                            }
                            Some((
                                path.file_stem()?.to_str()?.to_string(),
                                path.to_str()?.to_string(),
                            ))
                        })
                        .collect();
                    let db_name = db_name.clone();
                    let uri = uri.clone();
                    start_collection_import(state, client.clone(), db_name.clone(), files);
                    state.file_picker = None;
                    state.collection_to_load = Some((uri, db_name));
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

                    let uri = uri.clone();
                    start_database_import(
                        state,
                        client.clone(),
                        db_name.to_string(),
                        folder_path.to_str().unwrap().to_string(),
                    );
                    state.file_picker = None;
                    state.collection_to_load = Some((uri, db_name.to_string()));
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
