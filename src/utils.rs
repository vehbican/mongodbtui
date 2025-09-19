use crate::app::Connection;
use arboard::Clipboard;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

fn get_config_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("mongodbtui");
    std::fs::create_dir_all(&path).ok();
    path.push("connections.csv");
    path
}
pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mongodbtui")
}

pub fn update_connection(input: &str) -> std::io::Result<()> {
    let parts: Vec<&str> = input.trim().split(';').map(|s| s.trim()).collect();

    if parts.len() != 3 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid input format",
        ));
    }

    let id = parts[0].parse::<usize>().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "ID must be a number")
    })?;

    let uri = parts[1].to_string();
    let name = parts[2].to_string();

    if name.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Connection name cannot be empty",
        ));
    }

    if uri.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "URI cannot be empty",
        ));
    }

    let mut connections = crate::utils::load_connections().unwrap_or_default();
    connections.retain(|c| c.id != id);

    connections.push(crate::app::Connection { id, uri, name });

    crate::utils::overwrite_connections(&connections)?;

    Ok(())
}

pub fn overwrite_connections(conns: &[Connection]) -> std::io::Result<()> {
    let path = get_config_file_path();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for conn in conns {
        writeln!(file, "{};{};{}", conn.id, conn.uri, conn.name)?;
    }
    Ok(())
}

pub fn save_connection(uri: &str, name: &str) -> std::io::Result<()> {
    let mut max_id = 0;

    let path = get_config_file_path();
    if let Ok(file) = File::open(path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if let Some(first_part) = line.split(';').next() {
                if let Ok(id) = first_part.trim().parse::<usize>() {
                    if id > max_id {
                        max_id = id;
                    }
                }
            }
        }
    }

    let new_id = max_id + 1;

    let path = get_config_file_path();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    writeln!(file, "{};{};{}", new_id, uri, name)?;
    Ok(())
}
pub fn parse_connection_input(input: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.trim().split(';').map(|s| s.trim()).collect();

    if parts.len() == 2 {
        let uri = parts[0];
        let name = parts[1];

        if !uri.is_empty() && !name.is_empty() && name.len() >= 1 && uri.len() >= 1 {
            return Some((uri.to_string(), name.to_string()));
        }
    }

    None
}

pub fn load_connections() -> std::io::Result<Vec<Connection>> {
    let path = get_config_file_path();
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        let mut connections = Vec::new();

        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split(';').collect();
            if parts.len() == 3 {
                if let Ok(id) = parts[0].parse::<usize>() {
                    connections.push(Connection {
                        id,
                        uri: parts[1].to_string(),
                        name: parts[2].to_string(),
                    });
                }
            }
        }

        Ok(connections)
    } else {
        Ok(vec![])
    }
}
pub fn read_clipboard_string() -> Result<String, String> {
    let mut cb = Clipboard::new().map_err(|e| format!("Clipboard açılamadı: {e}"))?;
    cb.get_text()
        .map_err(|e| format!("Clipboard okunamadı: {e}"))
}
