use crate::db::import_export::import_collection;
use mongodb::Client;
use std::collections::HashSet;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

pub enum FileEntry {
    Real(DirEntry),
    Parent(PathBuf),
}

pub enum FilePickerMode {
    ImportCollection,
    ImportDatabase,
    RunScript,
}

pub struct FilePickerState {
    pub mode: FilePickerMode,
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub selected_files: HashSet<PathBuf>,
}

impl FilePickerState {
    pub fn new(mode: FilePickerMode, base_path: PathBuf) -> std::io::Result<Self> {
        let entries = Self::read_entries_with_parent(&base_path)?;
        Ok(Self {
            mode,
            current_path: base_path,
            entries,
            selected_index: 0,
            selected_files: HashSet::new(),
        })
    }
    pub async fn perform_import(
        &self,
        client: &Client,
        db_name: &str,
        _uri: &str,
    ) -> (usize, usize) {
        let mut success = 0;
        let mut failed = 0;

        for path in &self.selected_files {
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed");

                match import_collection(client, db_name, file_stem, path.to_str().unwrap()).await {
                    Ok(_) => success += 1,
                    Err(e) => {
                        failed += 1;
                        eprintln!("❌ Failed to import {}: {}", path.display(), e);
                    }
                }
            }
        }

        (success, failed)
    }

    fn read_entries_with_parent(path: &Path) -> std::io::Result<Vec<FileEntry>> {
        let mut entries = vec![];

        if let Some(parent) = path.parent() {
            entries.push(FileEntry::Parent(parent.to_path_buf()));
        }

        let mut dir_entries = std::fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(FileEntry::Real)
            .collect::<Vec<_>>();

        dir_entries.sort_by_key(|entry| match entry {
            FileEntry::Real(e) => e.file_name(),
            FileEntry::Parent(_) => std::ffi::OsString::from(".."),
        });

        entries.extend(dir_entries);
        Ok(entries)
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    pub fn enter_directory(&mut self, dir: &PathBuf) {
        if let Ok(entries) = Self::read_entries_with_parent(dir) {
            self.entries = entries;
            self.current_path = dir.clone();
            self.selected_index = 0;
        }
    }

    pub fn next(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.selected_entry() {
            if let Some(path) = match entry {
                FileEntry::Real(e) => Some(e.path()),
                FileEntry::Parent(_) => None,
            } {
                if self.selected_files.contains(&path) {
                    self.selected_files.remove(&path);
                } else {
                    self.selected_files.insert(path);
                }
            }
        }
    }
}
