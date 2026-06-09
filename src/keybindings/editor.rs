use crate::app::AppState;
use crate::db::client::apply_edited_json;
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use edit::edit;
use mongodb::bson::Document;
use std::io::{self, Stdout, Write};

pub async fn open_in_external_editor(state: &mut AppState) -> Result<(), String> {
    let (db_name, col_name) = state
        .selected_collection
        .as_ref()
        .map(|(_uri, db, col)| (db.clone(), col.clone()))
        .ok_or_else(|| "No collection selected.".to_string())?;

    let client = state
        .mongo_client
        .as_ref()
        .ok_or_else(|| "No MongoDB connection.".to_string())?;

    let original_doc: &Document = state
        .current_documents
        .get(state.selected_doc_index)
        .ok_or_else(|| "No document selected.".to_string())?;
    let original_id = original_doc.get("_id").cloned();

    let initial = serde_json::to_string_pretty(original_doc)
        .map_err(|e| format!("Document could not be converted to JSON: {e}"))?;

    let _guard = TuiSuspendGuard::suspend().map_err(|e| format!("Could not suspend TUI: {e}"))?;
    let edited = edit(&initial).map_err(|e| format!("Could not open external editor: {e}"))?;

    if edited.trim() == initial.trim() {
        state.redraw = true;
        return Ok(());
    }

    if let Err(e) = apply_edited_json(client, &db_name, &col_name, original_doc, &edited).await {
        state.popup_message = Some(format!("❌ Database update failed: {e}"));
        state.redraw = true;
        return Err(format!("{e}"));
    }

    match serde_json::from_str::<Document>(&edited) {
        Ok(mut new_doc) => {
            if let Some(id) = original_id {
                new_doc.insert("_id", id);
            }
            if let Some(slot) = state.current_documents.get_mut(state.selected_doc_index) {
                *slot = new_doc;
            }
            state.popup_message_success =
                Some("External editor changes saved and database updated ✅".into());
        }
        Err(_) => {
            state.popup_message_success =
                Some("Database updated ✅ (local parse failed; the view will refresh soon)".into());
        }
    }

    state.redraw = true;
    Ok(())
}

struct TuiSuspendGuard {
    stdout: Stdout,
}
impl TuiSuspendGuard {
    pub fn suspend() -> io::Result<Self> {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, DisableMouseCapture, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        Ok(Self { stdout })
    }
}
impl Drop for TuiSuspendGuard {
    fn drop(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(
            self.stdout.by_ref(),
            EnterAlternateScreen,
            EnableMouseCapture,
            Hide
        );
        let _ = self.stdout.flush();
    }
}
