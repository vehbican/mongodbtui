use crate::app::Connection;
use bson::Bson;
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
pub fn infer_bson_value(original: &Bson, input: &str) -> Bson {
    match original {
        Bson::Int32(_) => input
            .parse::<i32>()
            .map(Bson::Int32)
            .unwrap_or(Bson::String(input.to_string())),
        Bson::Int64(_) => input
            .parse::<i64>()
            .map(Bson::Int64)
            .unwrap_or(Bson::String(input.to_string())),
        Bson::Double(_) => input
            .parse::<f64>()
            .map(Bson::Double)
            .unwrap_or(Bson::String(input.to_string())),
        Bson::Boolean(_) => match input.to_lowercase().as_str() {
            "true" => Bson::Boolean(true),
            "false" => Bson::Boolean(false),
            _ => Bson::String(input.to_string()),
        },
        Bson::ObjectId(_) => bson::oid::ObjectId::parse_str(input)
            .map(Bson::ObjectId)
            .unwrap_or(Bson::String(input.to_string())),
        Bson::Null => {
            if input.trim().to_lowercase() == "null" {
                Bson::Null
            } else {
                Bson::String(input.to_string())
            }
        }
        Bson::Array(_) => {
            let items = input
                .split(',')
                .map(|s| Bson::String(s.trim().to_string()))
                .collect();
            Bson::Array(items)
        }
        Bson::Document(_) => match serde_json::from_str::<serde_json::Value>(input) {
            Ok(json) => bson::to_bson(&json).unwrap_or(Bson::String(input.to_string())),
            Err(_) => Bson::String(input.to_string()),
        },
        Bson::RegularExpression(_) => {
            if input.starts_with('/') && input.rfind('/').map_or(false, |i| i > 0) {
                let end = input.rfind('/').unwrap();
                let pattern = &input[1..end];
                let options = &input[end + 1..];
                Bson::RegularExpression(bson::Regex {
                    pattern: pattern.to_string(),
                    options: options.to_string(),
                })
            } else {
                Bson::String(input.to_string())
            }
        }
        _ => Bson::String(input.to_string()),
    }
}
