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
        .ok_or_else(|| "Herhangi bir koleksiyon seçili değil.".to_string())?;

    let client = state
        .mongo_client
        .as_ref()
        .ok_or_else(|| "MongoDB bağlantısı yok.".to_string())?;

    let original_doc: &Document = state
        .current_documents
        .get(state.selected_doc_index)
        .ok_or_else(|| "Herhangi bir doküman seçili değil.".to_string())?;

    let initial = serde_json::to_string_pretty(original_doc)
        .map_err(|e| format!("Doküman JSON'a çevrilemedi: {e}"))?;

    let _guard = TuiSuspendGuard::suspend().map_err(|e| format!("TUI askıya alınamadı: {e}"))?;
    let edited = edit(&initial).map_err(|e| format!("Dış editör açılamadı: {e}"))?;

    if edited.trim() == initial.trim() {
        state.popup_message_success = Some("Kaydedildi (değişiklik yok).".into());
        state.redraw = true;
        return Ok(());
    }

    if let Err(e) = apply_edited_json(client, &db_name, &col_name, original_doc, &edited).await {
        state.popup_message = Some(format!("❌ DB güncellemesi başarısız: {e}"));
        state.redraw = true;
        return Err(format!("{e}"));
    }

    match serde_json::from_str::<Document>(&edited) {
        Ok(new_doc) => {
            if let Some(slot) = state.current_documents.get_mut(state.selected_doc_index) {
                *slot = new_doc;
            }
            state.popup_message_success =
                Some("Dış editör değişiklikleri kaydedildi ve DB güncellendi ✅".into());
        }
        Err(_) => {
            state.popup_message_success = Some(
                "DB güncellendi ✅ (yerel parse hatası; görünüm yakında güncellenecek)".into(),
            );
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
