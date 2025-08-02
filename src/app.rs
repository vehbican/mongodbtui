use crate::{
    db::client::{count_documents, fetch_documents},
    tui::filepicker::FilePickerState,
};
use crossterm::event::KeyEvent;
use mongodb::{Client, bson::Document};
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;
#[derive(PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
    Editor,
}
#[derive(PartialEq)]
pub enum FocusArea {
    Connections,
    Documents,
    FilterSortInputs,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveInputField {
    Filter,
    Sort,
}

impl Default for FocusArea {
    fn default() -> Self {
        FocusArea::Connections
    }
}

#[derive(PartialEq)]
pub enum InputContext {
    Uri,
    ConnectionName,
    CollectionName,
    FieldEditEditor,
    None,
}
pub enum SelectableItem {
    Uri {
        uri: String,
        name: String,
        connected: bool,
    },
    Database {
        uri: String,
        name: String,
    },
    Collection {
        uri: String,
        db: String,
        name: String,
    },
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Normal
    }
}

impl Default for InputContext {
    fn default() -> Self {
        InputContext::None
    }
}

pub struct Connection {
    pub id: usize,
    pub uri: String,
    pub name: String,
}

pub struct AppState {
    pub connections: Vec<Connection>,
    pub input_text: String,
    pub mode: AppMode,
    pub input_context: InputContext,
    pub popup_message: Option<String>,
    pub popup_message_success: Option<String>,
    pub connected_uri: Option<String>,
    pub connect_to: Option<String>,
    pub mongo_client: Option<Client>,
    pub expanded_uris: HashSet<String>,
    pub tree_items: Vec<SelectableItem>,
    pub selected_index: usize,
    pub expanded_dbs: HashSet<(String, String)>,
    pub database_map: HashMap<String, Vec<String>>,
    pub collection_map: HashMap<(String, String), Vec<String>>,
    pub collection_to_load: Option<(String, String)>,
    pub db_to_expand: Option<(String, String)>,
    pub current_documents: Vec<Document>,
    pub selected_collection: Option<(String, String, String)>,
    pub fetch_collection_data: Option<(String, String, String)>,
    pub focus: FocusArea,
    pub selected_doc_index: usize,
    pub doc_scroll_offset: usize,
    pub document_skip: usize,
    pub document_limit: usize,
    pub filter_text: String,
    pub sort_text: String,
    pub active_input: Option<ActiveInputField>,
    pub document_counts: HashMap<(String, String, String), u64>,
    pub selected_field_index: usize,
    pub cursor_position: usize,
    pub input_graphemes: Vec<String>,
    pub last_key: Option<KeyEvent>,
    pub file_picker: Option<FilePickerState>,
    pub show_help: bool,
    pub help_scroll: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            input_text: String::new(),
            mode: AppMode::default(),
            input_context: InputContext::default(),
            popup_message: None,
            popup_message_success: None,
            connected_uri: None,
            connect_to: None,
            mongo_client: None,
            expanded_uris: HashSet::new(),
            tree_items: Vec::new(),
            selected_index: 0,
            expanded_dbs: HashSet::new(),
            database_map: HashMap::new(),
            collection_map: HashMap::new(),
            collection_to_load: None,
            db_to_expand: None,
            current_documents: Vec::new(),
            selected_collection: None,
            fetch_collection_data: None,
            focus: FocusArea::default(),
            selected_doc_index: 0,
            doc_scroll_offset: 0,
            document_skip: 0,
            document_limit: 100,
            filter_text: String::new(),
            sort_text: String::new(),
            active_input: Some(ActiveInputField::Filter),
            document_counts: HashMap::new(),
            selected_field_index: 0,
            cursor_position: 0,
            input_graphemes: Vec::new(),
            last_key: None,
            file_picker: None,
            show_help: false,
            help_scroll: 0,
        }
    }
}

impl AppState {
    pub fn field_count(&self) -> usize {
        self.current_documents
            .get(self.selected_doc_index)
            .map(|doc| doc.len())
            .unwrap_or(0)
    }

    pub fn next_field(&mut self) {
        let count = self.field_count();
        if count == 0 {
            return;
        }

        self.selected_field_index = (self.selected_field_index + 1) % count;
    }

    pub fn previous_field(&mut self) {
        if self.selected_field_index > 0 {
            self.selected_field_index -= 1;
        }
    }

    pub fn reset_field_index(&mut self) {
        self.selected_field_index = 0;
        self.selected_doc_index = 0;
        self.doc_scroll_offset = 0;
    }

    pub fn rebuild_tree_items(&mut self) {
        self.tree_items.clear();

        for conn in &self.connections {
            let connected = self.connected_uri.as_ref() == Some(&conn.uri);
            self.tree_items.push(SelectableItem::Uri {
                uri: conn.uri.clone(),
                name: conn.name.clone(),
                connected,
            });

            if self.expanded_uris.contains(&conn.uri) {
                if let Some(dbs) = self.database_map.get(&conn.uri) {
                    for db in dbs {
                        self.tree_items.push(SelectableItem::Database {
                            uri: conn.uri.clone(),
                            name: db.clone(),
                        });

                        if self.expanded_dbs.contains(&(conn.uri.clone(), db.clone())) {
                            if let Some(cols) =
                                self.collection_map.get(&(conn.uri.clone(), db.clone()))
                            {
                                for col in cols {
                                    self.tree_items.push(SelectableItem::Collection {
                                        uri: conn.uri.clone(),
                                        db: db.clone(),
                                        name: col.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    pub async fn reload_documents_for_selected_collection(&mut self) {
        if let (Some(client), Some((uri, db, collection))) =
            (&self.mongo_client, &self.selected_collection)
        {
            match count_documents(client, db, collection, &self.filter_text).await {
                Ok(count) => {
                    self.document_counts
                        .insert((uri.clone(), db.clone(), collection.clone()), count);
                }
                Err(e) => {
                    self.popup_message = Some(format!("⚠️ Could not count documents: {}", e));
                    return;
                }
            }

            match fetch_documents(
                client,
                db,
                collection,
                0,
                self.document_limit as u64,
                &self.filter_text,
                &self.sort_text,
            )
            .await
            {
                Ok(docs) => {
                    self.current_documents = docs;
                    self.document_skip = self.current_documents.len();
                }
                Err(e) => {
                    self.popup_message = Some(format!(
                        "❌ Failed to fetch documents for {}.{}: {}",
                        db, collection, e
                    ));
                }
            }
        }
    }
    pub fn update_graphemes(&mut self) {
        self.input_graphemes = self
            .input_text
            .graphemes(true)
            .map(|g| g.to_string())
            .collect();
    }
}
