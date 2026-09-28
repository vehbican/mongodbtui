use crate::app::ActiveInputField;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const MAX_ENTRIES: usize = 500;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryHistory {
    filter: Vec<String>,
    sort: Vec<String>,
}

impl QueryHistory {
    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mongodbtui/history.json")
    }

    pub fn load() -> io::Result<Self> {
        Self::load_from(&Self::path())
    }

    fn load_from(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::path())
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        serde_json::to_writer_pretty(options.open(path)?, self).map_err(io::Error::other)
    }

    pub fn record(&mut self, field: ActiveInputField, value: &str) -> bool {
        let value = value.trim();
        if value.is_empty()
            || value
                .chars()
                .filter(|c| !c.is_whitespace())
                .eq("{}".chars())
        {
            return false;
        }
        let entries = match field {
            ActiveInputField::Filter => &mut self.filter,
            ActiveInputField::Sort => &mut self.sort,
        };
        entries.retain(|entry| entry != value);
        entries.push(value.to_owned());
        if entries.len() > MAX_ENTRIES {
            entries.drain(..entries.len() - MAX_ENTRIES);
        }
        true
    }

    // Only complete at the end, or immediately before the closing object brace.
    // Preserve the suffix so editing in the middle never replaces existing input.
    pub fn suggestion(&self, field: ActiveInputField, value: &str, cursor: usize) -> Option<&str> {
        let len = value.chars().count();
        if cursor > len {
            return None;
        }
        let byte = value
            .char_indices()
            .nth(cursor)
            .map_or(value.len(), |(i, _)| i);
        let (prefix, suffix) = value.split_at(byte);
        if !suffix.is_empty() && !(suffix == "}" && value.starts_with('{')) {
            return None;
        }
        let entries = match field {
            ActiveInputField::Filter => &self.filter,
            ActiveInputField::Sort => &self.sort,
        };
        entries.iter().rev().find_map(|entry| {
            (entry.len() > value.len() && entry.starts_with(prefix) && entry.ends_with(suffix))
                .then_some(entry.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ActiveInputField::{Filter, Sort};

    #[test]
    fn newest_match_is_deduplicated_and_fields_are_separate() {
        let mut history = QueryHistory::default();
        history.record(Filter, "{\"name\": 1}");
        history.record(Filter, "{\"name\": 2}");
        history.record(Sort, "{\"name\": -1}");
        history.record(Filter, "{\"name\": 1}");
        assert_eq!(history.filter.len(), 2);
        assert_eq!(
            history.suggestion(Filter, "{\"n}", 3),
            Some("{\"name\": 1}")
        );
        assert_eq!(history.suggestion(Sort, "{\"n}", 3), Some("{\"name\": -1}"));
        assert_eq!(history.suggestion(Filter, "{\"x}", 3), None);
        assert_eq!(history.suggestion(Filter, "{\"name\": 1}", 11), None);
        assert_eq!(history.suggestion(Filter, "{\"n}", 2), None);
    }

    #[test]
    fn unicode_prefixes_and_unbraced_input_work() {
        let mut history = QueryHistory::default();
        history.record(Filter, "{\"şehir\": \"İzmir\"}");
        assert_eq!(
            history.suggestion(Filter, "{\"şe}", 4),
            Some("{\"şehir\": \"İzmir\"}")
        );
        assert_eq!(
            history.suggestion(Filter, "{\"şe", 4),
            Some("{\"şehir\": \"İzmir\"}")
        );
        assert_eq!(history.suggestion(Filter, "{}", 9), None);
    }

    #[test]
    fn empty_commands_are_ignored_and_history_is_bounded() {
        let mut history = QueryHistory::default();
        assert!(!history.record(Filter, " "));
        assert!(!history.record(Filter, "{ }"));
        for n in 0..=MAX_ENTRIES {
            history.record(Filter, &format!("{{\"n\": {n}}}"));
        }
        assert_eq!(history.filter.len(), MAX_ENTRIES);
        assert_eq!(history.filter[0], "{\"n\": 1}");
    }

    #[test]
    fn persistence_round_trip_and_invalid_file() {
        let path = std::env::temp_dir().join(format!(
            "mongodbtui-history-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(QueryHistory::load_from(&path).unwrap().filter.is_empty());
        let mut history = QueryHistory::default();
        history.record(Filter, "{\"active\": true}");
        history.record(Sort, "{\"date\": -1}");
        history.save_to(&path).unwrap();
        let loaded = QueryHistory::load_from(&path).unwrap();
        assert_eq!(loaded.filter, history.filter);
        assert_eq!(loaded.sort, history.sort);
        fs::write(&path, "invalid").unwrap();
        assert!(QueryHistory::load_from(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}
