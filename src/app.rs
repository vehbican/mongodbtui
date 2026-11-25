use crate::{
    db::client::{count_documents, fetch_documents},
    db::import_export::ExportProgress,
    theme::ThemeName,
    tui::filepicker::FilePickerState,
};
use crossterm::event::KeyEvent;
use mongodb::{
    Client,
    bson::{Document, oid::ObjectId},
};
use std::collections::{HashMap, HashSet};
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use unicode_segmentation::UnicodeSegmentation;
#[derive(PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
}
#[derive(PartialEq)]
pub enum FocusArea {
    Connections,
    Documents,
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
    NewConnection,
    ConnectionName,
    CollectionName,
    SearchCollections,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionFormField {
    Name,
    Uri,
    Color,
}

pub struct ConnectionForm {
    pub id: Option<usize>,
    pub name: String,
    pub uri: String,
    pub color: String,
    pub active_field: ConnectionFormField,
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

pub enum PendingDeletion {
    Connection {
        id: usize,
        uri: String,
        name: String,
    },
    Collection {
        uri: String,
        db: String,
        name: String,
    },
    Database {
        uri: String,
        name: String,
    },
    Document {
        db: String,
        collection: String,
        id: ObjectId,
    },
    Field {
        db: String,
        collection: String,
        id: ObjectId,
        name: String,
    },
}

pub struct PendingBulkUpdate {
    pub db: String,
    pub collection: String,
    pub filter: Document,
    pub update: Document,
    pub matched_count: u64,
}

pub struct PendingBulkDeletion {
    pub db: String,
    pub collection: String,
    pub filter: Document,
    pub matched_count: u64,
}

impl PendingBulkDeletion {
    pub fn confirmation_message(&self) -> String {
        format!(
            "Delete {} document(s) matching this filter? [y/N]",
            self.matched_count
        )
    }
}

impl PendingBulkUpdate {
    pub fn confirmation_message(&self) -> String {
        format!(
            "Update {} document(s) matching this filter? [y/N]",
            self.matched_count
        )
    }
}

impl PendingDeletion {
    pub fn confirmation_message(&self) -> String {
        let target = match self {
            Self::Connection { name, .. } => format!("connection '{name}'"),
            Self::Collection { name, .. } => format!("collection '{name}'"),
            Self::Database { name, .. } => format!("database '{name}'"),
            Self::Document { .. } => "selected document".to_string(),
            Self::Field { name, .. } => format!("field '{name}'"),
        };

        format!("Are you sure you want to delete {target}? [y/N]")
    }
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
    pub color: String,
}

pub struct CollectionTab {
    pub key: (String, String, String),
    pub documents: Vec<Document>,
    pub loaded: bool,
    pub selected_doc_index: usize,
    pub doc_scroll_offset: usize,
    pub document_line_scroll: u16,
    pub document_skip: usize,
    pub filter_text: String,
    pub sort_text: String,
    pub selected_field_index: usize,
    pub expanded_field: Option<(usize, usize)>,
}

impl CollectionTab {
    fn new(key: (String, String, String)) -> Self {
        Self {
            key,
            documents: Vec::new(),
            loaded: false,
            selected_doc_index: 0,
            doc_scroll_offset: 0,
            document_line_scroll: 0,
            document_skip: 0,
            filter_text: "{}".to_string(),
            sort_text: "{}".to_string(),
            selected_field_index: 0,
            expanded_field: None,
        }
    }
}

pub struct AppState {
    pub command_palette: Option<crate::command_palette::CommandPalette>,
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
    pub document_line_scroll: u16,
    pub document_skip: usize,
    pub document_limit: usize,
    pub filter_text: String,
    pub sort_text: String,
    pub query_history: crate::history::QueryHistory,
    pub active_input: Option<ActiveInputField>,
    pub document_counts: HashMap<(String, String, String), u64>,
    pub selected_field_index: usize,
    pub expanded_field: Option<(usize, usize)>,
    pub cursor_position: usize,
    pub input_graphemes: Vec<String>,
    pub last_key: Option<KeyEvent>,
    pub file_picker: Option<FilePickerState>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub redraw: bool,
    pub collection_search_hits: Vec<(String, String, String)>,
    pub collection_search_idx: usize,
    pub theme: ThemeName,
    pub pending_deletion: Option<PendingDeletion>,
    pub pending_bulk_update: Option<PendingBulkUpdate>,
    pub pending_bulk_deletion: Option<PendingBulkDeletion>,
    pub export_progress: Option<ExportProgress>,
    pub export_progress_rx: Option<UnboundedReceiver<ExportProgress>>,
    pub export_task: Option<JoinHandle<Result<(), String>>>,
    pub connection_form: Option<ConnectionForm>,
    pub collection_tabs: Vec<CollectionTab>,
    pub active_tab: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            command_palette: None,
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
            document_line_scroll: 0,
            document_skip: 0,
            document_limit: 100,
            filter_text: "{}".to_string(),
            sort_text: "{}".to_string(),
            query_history: crate::history::QueryHistory::default(),
            active_input: Some(ActiveInputField::Filter),
            document_counts: HashMap::new(),
            selected_field_index: 0,
            expanded_field: None,
            cursor_position: 1,
            input_graphemes: Vec::new(),
            last_key: None,
            file_picker: None,
            show_help: false,
            help_scroll: 0,
            redraw: false,
            collection_search_hits: Vec::new(),
            collection_search_idx: 0,
            theme: ThemeName::default(),
            pending_deletion: None,
            pending_bulk_update: None,
            pending_bulk_deletion: None,
            export_progress: None,
            export_progress_rx: None,
            export_task: None,
            connection_form: None,
            collection_tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

impl AppState {
    pub fn sync_connection_form_input(&mut self) {
        let Some(form) = self.connection_form.as_mut() else {
            return;
        };
        match form.active_field {
            ConnectionFormField::Name => form.name = self.input_text.clone(),
            ConnectionFormField::Uri => form.uri = self.input_text.clone(),
            ConnectionFormField::Color => form.color = self.input_text.clone(),
        }
    }

    pub fn select_connection_form_field(&mut self, field: ConnectionFormField) {
        self.sync_connection_form_input();
        let Some(form) = self.connection_form.as_mut() else {
            return;
        };
        form.active_field = field;
        self.input_text = match field {
            ConnectionFormField::Name => form.name.clone(),
            ConnectionFormField::Uri => form.uri.clone(),
            ConnectionFormField::Color => form.color.clone(),
        };
        self.cursor_position = self.input_text.chars().count();
    }

    pub fn save_active_collection_tab(&mut self) {
        let Some((uri, db, collection)) = self.selected_collection.clone() else {
            return;
        };
        let key = (uri, db, collection);
        let Some(tab) = self.collection_tabs.iter_mut().find(|tab| tab.key == key) else {
            return;
        };
        tab.documents = self.current_documents.clone();
        tab.selected_doc_index = self.selected_doc_index;
        tab.doc_scroll_offset = self.doc_scroll_offset;
        tab.document_line_scroll = self.document_line_scroll;
        tab.document_skip = self.document_skip;
        tab.filter_text = self.filter_text.clone();
        tab.sort_text = self.sort_text.clone();
        tab.selected_field_index = self.selected_field_index;
        tab.expanded_field = self.expanded_field;
    }

    pub fn activate_collection(&mut self, key: (String, String, String)) {
        self.save_active_collection_tab();
        let tab_index = self
            .collection_tabs
            .iter()
            .position(|tab| tab.key == key)
            .unwrap_or_else(|| {
                self.collection_tabs.push(CollectionTab::new(key.clone()));
                self.collection_tabs.len() - 1
            });
        self.active_tab = tab_index;
        let tab = &self.collection_tabs[tab_index];
        self.selected_collection = Some(key);
        self.current_documents = tab.documents.clone();
        self.selected_doc_index = tab.selected_doc_index;
        self.doc_scroll_offset = tab.doc_scroll_offset;
        self.document_line_scroll = tab.document_line_scroll;
        self.document_skip = tab.document_skip;
        self.filter_text = tab.filter_text.clone();
        self.sort_text = tab.sort_text.clone();
        self.selected_field_index = tab.selected_field_index;
        self.expanded_field = tab.expanded_field;
        self.fetch_collection_data = if tab.loaded {
            None
        } else {
            self.selected_collection.clone()
        };
    }

    pub fn mark_active_collection_loaded(&mut self) {
        if let Some(tab) = self.collection_tabs.get_mut(self.active_tab) {
            tab.loaded = true;
        }
        self.save_active_collection_tab();
    }

    pub fn switch_collection_tab(&mut self, offset: isize) {
        if self.collection_tabs.len() < 2 {
            return;
        }
        self.save_active_collection_tab();
        self.active_tab = (self.active_tab as isize + offset)
            .rem_euclid(self.collection_tabs.len() as isize) as usize;
        let key = self.collection_tabs[self.active_tab].key.clone();
        self.activate_collection(key);
    }

    pub fn close_active_collection_tab(&mut self) {
        if self.collection_tabs.is_empty() {
            return;
        }

        self.save_active_collection_tab();
        self.collection_tabs.remove(self.active_tab);
        self.fetch_collection_data = None;

        if self.collection_tabs.is_empty() {
            self.active_tab = 0;
            self.selected_collection = None;
            self.current_documents.clear();
            self.document_counts.clear();
            return;
        }

        self.active_tab = self.active_tab.min(self.collection_tabs.len() - 1);
        let key = self.collection_tabs[self.active_tab].key.clone();
        self.activate_collection(key);
    }

    pub fn query_suggestion(&self) -> Option<&str> {
        if self.mode != AppMode::Insert || self.input_context != InputContext::None {
            return None;
        }
        let field = self.active_input?;
        let value = match field {
            ActiveInputField::Filter => &self.filter_text,
            ActiveInputField::Sort => &self.sort_text,
        };
        self.query_history
            .suggestion(field, value, self.cursor_position)
    }

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

    pub fn toggle_selected_field_expansion(&mut self) {
        let selected = (self.selected_doc_index, self.selected_field_index);
        self.expanded_field = if self.expanded_field == Some(selected) {
            None
        } else {
            Some(selected)
        };
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
                    self.expanded_field = None;
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
