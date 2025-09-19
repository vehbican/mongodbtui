use crate::app::{ActiveInputField, AppMode, AppState, FocusArea, InputContext, SelectableItem};
use crate::keybindings::editor::open_in_external_editor;
use crate::tui::events::{goto_collection, inner_end_pos};
use crate::tui::filepicker::{FilePickerMode, FilePickerState};
use crate::utils::read_clipboard_string;
use crate::widgets::help_popup::HELP_TEXT;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

pub async fn handle_normal(key: KeyEvent, state: &mut AppState) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,

        KeyCode::Char('o') => {
            state.mode = AppMode::Insert;
            state.input_context = InputContext::Uri;
            state.input_text.clear();
            state.cursor_position = state.input_text.chars().count();
        }

        KeyCode::Char('?') => {
            state.show_help = !state.show_help;
            state.help_scroll = 0;
            state.current_documents.clear();
        }

        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match (c, &state.focus) {
                ('l', FocusArea::Connections) => state.focus = FocusArea::FilterSortInputs,
                ('j', FocusArea::FilterSortInputs) => state.focus = FocusArea::Documents,
                ('k', FocusArea::Documents) => state.focus = FocusArea::FilterSortInputs,
                ('h', FocusArea::Documents) | ('h', FocusArea::FilterSortInputs) => {
                    state.focus = FocusArea::Connections
                }
                _ => {}
            }
        }

        KeyCode::Char('/') if state.focus == FocusArea::Connections => {
            state.mode = AppMode::Insert;
            state.input_context = InputContext::SearchCollections;
            state.input_text.clear();
            state.cursor_position = 0;
        }
        KeyCode::Tab => {
            if state.focus == FocusArea::FilterSortInputs {
                state.active_input = match state.active_input {
                    Some(ActiveInputField::Filter) => Some(ActiveInputField::Sort),
                    Some(ActiveInputField::Sort) => Some(ActiveInputField::Filter),
                    _ => Some(ActiveInputField::Filter),
                };
                state.cursor_position = match state.active_input {
                    Some(ActiveInputField::Filter) => inner_end_pos(&state.filter_text),
                    Some(ActiveInputField::Sort) => inner_end_pos(&state.sort_text),
                    _ => 0,
                };
            }
        }

        KeyCode::Char('i') => {
            let Some(SelectableItem::Database {
                uri: _,
                name: _db_name,
            }) = state.tree_items.get(state.selected_index)
            else {
                state.popup_message =
                    Some("❗ You must select a database to import into.".to_string());
                return false;
            };

            let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match FilePickerState::new(FilePickerMode::ImportCollection, path) {
                Ok(picker) => {
                    state.file_picker = Some(picker);
                    state.current_documents.clear();
                }
                Err(e) => {
                    state.popup_message = Some(format!("❌ Failed to open file picker: {}", e));
                }
            }
        }
        KeyCode::Char('f') => {
            let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match FilePickerState::new(FilePickerMode::RunScript, path) {
                Ok(picker) => {
                    state.file_picker = Some(picker);
                }
                Err(e) => {
                    state.popup_message = Some(format!("❌ Failed to open file picker: {}", e));
                }
            }
        }

        KeyCode::Char('I') => {
            let Some(SelectableItem::Uri { uri: _, .. }) =
                state.tree_items.get(state.selected_index)
            else {
                state.popup_message =
                    Some("❗ You must select a connection to import a database.".to_string());
                return false;
            };

            let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            match FilePickerState::new(FilePickerMode::ImportDatabase, path) {
                Ok(picker) => {
                    state.file_picker = Some(picker);
                    state.current_documents.clear();
                }
                Err(e) => {
                    state.popup_message = Some(format!("❌ Failed to open file picker: {}", e));
                }
            }
        }

        KeyCode::Char('d') => {
            if let Some(KeyEvent {
                code: KeyCode::Char('d'),
                ..
            }) = state.last_key
            {
                match state.focus {
                    FocusArea::Connections => {
                        if let Some(item) = state.tree_items.get(state.selected_index) {
                            match item {
                                SelectableItem::Collection { db, name, .. } => {
                                    if let Some(client) = &state.mongo_client {
                                        match crate::db::client::delete_collection(client, db, name)
                                            .await
                                        {
                                            Ok(_) => {
                                                state.popup_message_success = Some(format!(
                                                    "✅ Deleted collection: {}",
                                                    name
                                                ));
                                                state.collection_to_load = Some((
                                                    state.connected_uri.clone().unwrap(),
                                                    db.clone(),
                                                ));
                                            }
                                            Err(e) => {
                                                state.popup_message = Some(format!(
                                                    "❌ Failed to delete collection: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }

                                SelectableItem::Database { uri, name: db_name } => {
                                    if let Some(client) = &state.mongo_client {
                                        match crate::db::client::delete_database(client, db_name)
                                            .await
                                        {
                                            Ok(_) => {
                                                state.popup_message_success = Some(format!(
                                                    "✅ Deleted database: {}",
                                                    db_name
                                                ));
                                                state.db_to_expand =
                                                    Some((uri.clone(), db_name.clone()));
                                            }
                                            Err(e) => {
                                                state.popup_message = Some(format!(
                                                    "❌ Failed to delete database: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }

                                _ => {}
                            }
                        }
                    }
                    FocusArea::Documents => {
                        if let Some(doc) = state.current_documents.get(state.selected_doc_index) {
                            if let Some(id) = doc.get_object_id("_id").ok() {
                                if let Some((_, db, col)) = &state.selected_collection {
                                    if let Some(client) = &state.mongo_client {
                                        match crate::db::client::delete_document_by_id(
                                            client, db, col, id,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                state
                                                    .reload_documents_for_selected_collection()
                                                    .await;
                                                state.popup_message_success =
                                                    Some("✅ Document deleted".to_string());
                                            }
                                            Err(e) => {
                                                state.popup_message = Some(format!(
                                                    "❌ Failed to delete document: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    FocusArea::FilterSortInputs => {
                        if let Some((_, db, col)) = &state.selected_collection {
                            if let Some(client) = &state.mongo_client {
                                match crate::db::client::delete_documents_by_filter(
                                    client,
                                    db,
                                    col,
                                    &state.filter_text,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        state.reload_documents_for_selected_collection().await;
                                        state.popup_message_success =
                                            Some("✅ Filtered documents deleted".to_string());
                                    }
                                    Err(e) => {
                                        state.popup_message = Some(format!(
                                            "❌ Failed to delete filtered documents: {}",
                                            e
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                state.last_key = None;
            } else {
                state.last_key = Some(key);
            }
        }

        KeyCode::Char('D') => {
            if state.focus == FocusArea::Documents {
                let maybe_id_and_field = {
                    if let Some(doc) = state.current_documents.get(state.selected_doc_index) {
                        if let Some((field, _)) = doc.iter().nth(state.selected_field_index) {
                            if field == "_id" {
                                state.popup_message =
                                    Some("❌ Cannot delete _id field.".to_string());
                                return false;
                            }
                            if let Some(id) = doc.get_object_id("_id").ok() {
                                Some((field.to_string(), id))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some((field, id)) = maybe_id_and_field {
                    if let Some((_, db, col)) = &state.selected_collection {
                        if let Some(client) = &state.mongo_client {
                            match crate::db::client::delete_field_in_document(
                                client, db, col, id, &field,
                            )
                            .await
                            {
                                Ok(_) => {
                                    state.reload_documents_for_selected_collection().await;
                                    state.popup_message_success =
                                        Some(format!("✅ Deleted field: {}", field));
                                }
                                Err(e) => {
                                    state.popup_message =
                                        Some(format!("❌ Failed to delete field: {}", e));
                                }
                            }
                        }
                    }
                }
            }
        }

        KeyCode::Char('n') => {
            if state.focus == FocusArea::Documents {
                state.next_field();
            } else if state.focus == FocusArea::Connections
                && !state.collection_search_hits.is_empty()
            {
                state.collection_search_idx =
                    (state.collection_search_idx + 1) % state.collection_search_hits.len();

                if let Some((uri, db, name)) = state
                    .collection_search_hits
                    .get(state.collection_search_idx)
                    .cloned()
                {
                    goto_collection(state, &uri, &db, &name);
                }
            }
        }

        KeyCode::Char('N') => {
            if state.focus == FocusArea::Documents {
                state.previous_field();
            } else if state.focus == FocusArea::Connections
                && !state.collection_search_hits.is_empty()
            {
                if state.collection_search_idx == 0 {
                    state.collection_search_idx = state.collection_search_hits.len() - 1;
                } else {
                    state.collection_search_idx -= 1;
                }

                if let Some((uri, db, name)) = state
                    .collection_search_hits
                    .get(state.collection_search_idx)
                    .cloned()
                {
                    goto_collection(state, &uri, &db, &name);
                }
            }
        }

        KeyCode::Char('e') => match state.focus {
            FocusArea::Documents => {
                if state
                    .current_documents
                    .get(state.selected_doc_index)
                    .is_some()
                {
                    if let Err(msg) = open_in_external_editor(state).await {
                        state.popup_message = Some(format!("❌ {}", msg));
                    }
                }
            }
            _ => {
                if let Some(item) = state.tree_items.get(state.selected_index) {
                    match item {
                        SelectableItem::Uri { uri, .. } => {
                            if let Some(conn) = state.connections.iter().find(|c| &c.uri == uri) {
                                state.mode = AppMode::Insert;
                                state.input_context = InputContext::ConnectionName;
                                state.input_text =
                                    format!("{};{};{}", conn.id, conn.uri, conn.name);
                                state.cursor_position = state.input_text.chars().count();
                            }
                        }
                        SelectableItem::Collection { uri, db, name } => {
                            state.mode = AppMode::Insert;
                            state.input_context = InputContext::CollectionName;
                            state.input_text = name.clone();
                            state.selected_collection =
                                Some((uri.clone(), db.clone(), name.clone()));
                            state.cursor_position = state.input_text.chars().count();
                        }
                        _ => {}
                    }
                }
            }
        },

        KeyCode::Char('a') => {
            if state.focus == FocusArea::FilterSortInputs {
                state.mode = AppMode::Insert;
                state.active_input = Some(
                    state
                        .active_input
                        .as_ref()
                        .copied()
                        .unwrap_or(ActiveInputField::Filter),
                );

                state.input_context = InputContext::None;
                match state.active_input {
                    Some(ActiveInputField::Filter) => {
                        state.input_text = state.filter_text.clone();
                        state.cursor_position = state.input_text.chars().count();
                    }
                    Some(ActiveInputField::Sort) => {
                        state.input_text = state.sort_text.clone();
                        state.cursor_position = state.input_text.chars().count();
                    }
                    _ => {}
                }
            }
        }

        KeyCode::Esc => {
            state.popup_message = None;
            state.popup_message_success = None;
            state.last_key = None;
            state.input_text.clear();
            state.cursor_position = 0;
            state.collection_search_hits.clear();
            state.collection_search_idx = 0;
        }
        KeyCode::Char('x') => {
            if let Some(item) = state.tree_items.get(state.selected_index) {
                if let Some(client) = &state.mongo_client {
                    match item {
                        SelectableItem::Collection { uri: _, db, name } => {
                            let path =
                                crate::utils::get_data_dir().join(format!("{}_{}.json", db, name));
                            match crate::db::import_export::export_collection(
                                client,
                                db,
                                name,
                                path.to_str().unwrap(),
                            )
                            .await
                            {
                                Ok(_) => {
                                    state.popup_message_success =
                                        Some(format!("✅ Exported collection: {}", path.display()))
                                }
                                Err(e) => {
                                    state.popup_message = Some(format!("❌ Export failed: {}", e))
                                }
                            }
                        }

                        SelectableItem::Database {
                            uri: _,
                            name: db_name,
                        } => {
                            let path = crate::utils::get_data_dir().join(db_name);
                            match crate::db::import_export::export_database(
                                client,
                                db_name,
                                path.to_str().unwrap(),
                            )
                            .await
                            {
                                Ok(_) => {
                                    state.popup_message_success =
                                        Some(format!("✅ Exported database to: {}", path.display()))
                                }
                                Err(e) => {
                                    state.popup_message = Some(format!("❌ Export failed: {}", e))
                                }
                            }
                        }

                        _ => {
                            state.popup_message = Some(
                                "⚠️ Only collections or databases can be exported.".to_string(),
                            );
                        }
                    }
                }
            }
        }

        KeyCode::Char('j') | KeyCode::Down => {
            if state.show_help {
                let total_lines = HELP_TEXT.lines().count();
                let visible_lines = 20;
                if state.help_scroll + 1 < total_lines.saturating_sub(visible_lines) {
                    state.help_scroll += 1;
                }
            } else {
                let max_visible = 1;
                match state.focus {
                    FocusArea::Connections => {
                        if state.selected_index + 1 < state.tree_items.len() {
                            state.selected_index += 1;
                        }
                    }
                    FocusArea::Documents => {
                        let total_loaded = state.current_documents.len();

                        if state.selected_doc_index + 1 < total_loaded {
                            state.selected_doc_index += 1;
                            if state.selected_doc_index >= state.doc_scroll_offset + max_visible {
                                state.doc_scroll_offset += 1;
                            }
                        } else {
                            if let Some((uri, db, name)) = &state.selected_collection {
                                let key = (uri.clone(), db.clone(), name.clone());

                                if let Some(count) = state.document_counts.get(&key) {
                                    if state.current_documents.len() >= *count as usize {
                                        return false;
                                    }
                                }

                                state.fetch_collection_data =
                                    Some((uri.clone(), db.clone(), name.clone()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        KeyCode::Char('k') | KeyCode::Up => {
            if state.show_help && state.help_scroll > 0 {
                state.help_scroll -= 1;
            } else {
                match state.focus {
                    FocusArea::Connections => {
                        if state.selected_index > 0 {
                            state.selected_index -= 1;
                        }
                    }
                    FocusArea::Documents => {
                        if state.selected_doc_index > 0 {
                            state.selected_doc_index -= 1;
                            if state.selected_doc_index < state.doc_scroll_offset {
                                state.doc_scroll_offset = state.doc_scroll_offset.saturating_sub(1);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('p') => {
            if state.focus == FocusArea::FilterSortInputs {
                match read_clipboard_string() {
                    Ok(mut clip) => {
                        if clip.ends_with('\n') {
                            clip.pop();
                        }
                        match state.active_input.unwrap_or(ActiveInputField::Filter) {
                            ActiveInputField::Filter => {
                                if state.filter_text.is_empty() {
                                    state.filter_text = clip;
                                } else {
                                    state.filter_text.push_str(&clip);
                                }
                                state.cursor_position = state.filter_text.chars().count();
                            }
                            ActiveInputField::Sort => {
                                if state.sort_text.is_empty() {
                                    state.sort_text = clip;
                                } else {
                                    state.sort_text.push_str(&clip);
                                }
                                state.cursor_position = state.sort_text.chars().count();
                            }
                        }
                    }
                    Err(e) => {
                        state.popup_message = Some(format!("❌ Yapıştırma başarısız: {e}"));
                    }
                }
            }
        }

        KeyCode::Enter => match state.focus {
            FocusArea::FilterSortInputs => {
                if let Some((uri, db, name)) = &state.selected_collection {
                    state.current_documents.clear();
                    state.document_skip = 0;
                    state.fetch_collection_data = Some((uri.clone(), db.clone(), name.clone()));
                }
            }

            FocusArea::Connections => {
                if let Some(selected_item) = state.tree_items.get(state.selected_index) {
                    match selected_item {
                        SelectableItem::Uri { uri, .. } => {
                            if let Some(current_uri) = state.connected_uri.clone() {
                                if current_uri != *uri {
                                    state.mongo_client = None;
                                    state.connected_uri = None;
                                    state.database_map.remove(&current_uri);
                                    state.expanded_uris.remove(&current_uri);
                                    state
                                        .expanded_dbs
                                        .retain(|(db_uri, _)| db_uri != &current_uri);
                                    state
                                        .collection_map
                                        .retain(|(db_uri, _), _| db_uri != &current_uri);
                                }
                            }

                            state.connect_to = Some(uri.clone());
                        }

                        SelectableItem::Database { uri, name: db_name } => {
                            let key = (uri.clone(), db_name.clone());
                            if state.expanded_dbs.contains(&key) {
                                state.expanded_dbs.remove(&key);
                                state.rebuild_tree_items();
                            } else {
                                state.collection_to_load = Some((uri.clone(), db_name.clone()));
                            }
                        }

                        SelectableItem::Collection { uri, db, name } => {
                            state.current_documents.clear();
                            state.document_skip = 0;
                            state.selected_collection =
                                Some((uri.clone(), db.clone(), name.clone()));
                            state.fetch_collection_data =
                                Some((uri.clone(), db.clone(), name.clone()));
                            state.focus = FocusArea::Documents;
                            state.selected_doc_index = 0;
                            state.doc_scroll_offset = 0;
                            state.filter_text = "{}".to_string();
                            state.sort_text = "{}".to_string();
                            state.cursor_position = 1;
                            state.reset_field_index();
                        }
                    }
                }
            }

            _ => {}
        },
        _ => {}
    }
    false
}
