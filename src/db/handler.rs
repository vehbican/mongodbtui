use crate::app::AppState;
use crate::db;
use std::string::ToString;

pub async fn handle_connection(state: &mut AppState, uri: &str) {
    state.connect_to = None;
    state.mongo_client = None;
    state.connected_uri = None;

    match db::client::connect_to_uri(uri).await {
        Ok(client) => {
            state.mongo_client = Some(client.clone());
            state.connected_uri = Some(uri.to_string());

            if let Ok(dbs) = db::client::list_databases(&client).await {
                state.database_map.insert(uri.to_string(), dbs);
                state.expanded_uris.insert(uri.to_string());
                state.rebuild_tree_items();
            }

            if let Some((db_uri, db_name)) = state.db_to_expand.clone() {
                if db_uri == uri {
                    if let Ok(cols) = db::client::list_collections(&client, &db_name).await {
                        state
                            .collection_map
                            .insert((db_uri.clone(), db_name.clone()), cols);
                        state.expanded_dbs.insert((db_uri, db_name));
                        state.rebuild_tree_items();
                    }
                }
            }
        }
        Err(err) => {
            state.mongo_client = None;
            state.connected_uri = None;
            state.database_map.remove(uri);
            state.expanded_uris.remove(uri);
            state.expanded_dbs.retain(|(db_uri, _)| db_uri != uri);
            state.collection_map.retain(|(db_uri, _), _| db_uri != uri);
            state.current_documents.clear();
            state.selected_collection = None;
            state.selected_doc_index = 0;
            state.doc_scroll_offset = 0;
            state.rebuild_tree_items();

            state.popup_message = Some(format!("❌ Connection failed to {}: {}", uri, err));
        }
    }

    state.db_to_expand = None;
}
pub async fn handle_collection_listing(state: &mut AppState, db_uri: &str, db_name: &str) {
    if let Some(client) = &state.mongo_client {
        match db::client::list_collections(client, db_name).await {
            Ok(cols) => {
                state
                    .collection_map
                    .insert((db_uri.to_string(), db_name.to_string()), cols);
                state
                    .expanded_dbs
                    .insert((db_uri.to_string(), db_name.to_string()));
                state.rebuild_tree_items();
            }
            Err(e) => {
                state.popup_message = Some(format!(
                    "Failed to list collections for {}.{}: {}",
                    db_uri, db_name, e
                ));
            }
        }
    }
    state.collection_to_load = None;
}

pub async fn fetch_and_update_documents(state: &mut AppState, uri: &str, db: &str, name: &str) {
    if let Some(client) = &state.mongo_client {
        match db::client::count_documents(client, db, name, &state.filter_text).await {
            Ok(count) => {
                state
                    .document_counts
                    .insert((uri.to_string(), db.to_string(), name.to_string()), count);
            }
            Err(e) => {
                state.popup_message = Some(format!("⚠️ Could not count documents: {}", e));
                return;
            }
        }

        match db::client::fetch_documents(
            client,
            db,
            name,
            state.document_skip as u64,
            state.document_limit as u64,
            &state.filter_text,
            &state.sort_text,
        )
        .await
        {
            Ok(new_docs) => {
                let fetched = new_docs.len();
                if !state.filter_text.trim().is_empty() {
                    state.selected_doc_index = 0;
                    state.doc_scroll_offset = 0;
                }

                if fetched > 0 {
                    state.current_documents.extend(new_docs);
                    state.document_skip += state.document_limit;
                }
                state.selected_collection =
                    Some((uri.to_string(), db.to_string(), name.to_string()));
            }
            Err(e) => {
                state.popup_message = Some(format!(
                    "Failed to fetch documents for {}.{}: {}",
                    db, name, e
                ));
            }
        }
    }

    state.fetch_collection_data = None;
}
