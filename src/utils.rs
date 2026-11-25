use crate::{app::Connection, theme::ThemeName};
use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use base64::Engine;
use keyring::Entry;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

const KEYRING_SERVICE: &str = "mongodbtui";
const PASSWORD_SENTINEL: &str = "__MONGODBTUI_KEYRING__";
pub const CONNECTION_COLOR_PRESETS: [&str; 6] = [
    "#E53935", "#FB8C00", "#FDD835", "#43A047", "#1E88E5", "#8E24AA",
];

fn get_config_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("mongodbtui");
    std::fs::create_dir_all(&path).ok();
    path.push("connections.csv");
    path
}

fn get_theme_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("mongodbtui");
    std::fs::create_dir_all(&path).ok();
    path.push("theme");
    path
}

fn get_connection_colors_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("mongodbtui");
    std::fs::create_dir_all(&path).ok();
    path.push("connection_colors");
    path
}

pub fn load_theme() -> ThemeName {
    std::fs::read_to_string(get_theme_file_path())
        .ok()
        .and_then(|value| ThemeName::parse(&value))
        .unwrap_or_default()
}

pub fn save_theme(theme: ThemeName) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(get_theme_file_path())?;
    writeln!(file, "{}", theme.as_str())
}

fn keyring_key(id: usize) -> String {
    format!("connection-{id}")
}

fn keyring_entry(id: usize) -> io::Result<Entry> {
    Entry::new(KEYRING_SERVICE, &keyring_key(id)).map_err(|e| io::Error::other(e.to_string()))
}

fn redact_uri_password(uri: &str) -> Option<(String, String)> {
    let scheme_end = uri.find("://")? + 3;
    let rest = &uri[scheme_end..];
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let at = authority.rfind('@')?;
    let userinfo = &authority[..at];
    let password_start = userinfo.find(':')? + 1;

    if userinfo[password_start..].is_empty() {
        return None;
    }

    let password_abs_start = scheme_end + password_start;
    let password_abs_end = scheme_end + at;
    let mut redacted = String::new();
    redacted.push_str(&uri[..password_abs_start]);
    redacted.push_str(PASSWORD_SENTINEL);
    redacted.push_str(&uri[password_abs_end..]);

    Some((
        redacted,
        uri[password_abs_start..password_abs_end].to_string(),
    ))
}

fn restore_uri_password(uri: &str, id: usize) -> io::Result<String> {
    if !uri.contains(PASSWORD_SENTINEL) {
        return Ok(uri.to_string());
    }

    let password = keyring_entry(id)?
        .get_password()
        .map_err(|e| io::Error::other(format!("Could not read keyring password: {e}")))?;
    Ok(uri.replace(PASSWORD_SENTINEL, &password))
}

fn secure_connection_for_storage(conn: &Connection) -> io::Result<Connection> {
    if conn.uri.contains(PASSWORD_SENTINEL) {
        return Ok(Connection {
            id: conn.id,
            uri: conn.uri.clone(),
            name: conn.name.clone(),
            color: conn.color.clone(),
        });
    }

    let Some((redacted_uri, password)) = redact_uri_password(&conn.uri) else {
        return Ok(Connection {
            id: conn.id,
            uri: conn.uri.clone(),
            name: conn.name.clone(),
            color: conn.color.clone(),
        });
    };

    keyring_entry(conn.id)?
        .set_password(&password)
        .map_err(|e| io::Error::other(format!("Could not write keyring password: {e}")))?;

    Ok(Connection {
        id: conn.id,
        uri: redacted_uri,
        name: conn.name.clone(),
        color: conn.color.clone(),
    })
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
    let color = connections
        .iter()
        .find(|connection| connection.id == id)
        .map(|connection| connection.color.clone())
        .unwrap_or_else(|| "#00D084".to_string());
    connections.retain(|c| c.id != id);

    connections.push(crate::app::Connection {
        id,
        uri,
        name,
        color,
    });

    crate::utils::overwrite_connections(&connections)?;

    Ok(())
}

pub fn overwrite_connections(conns: &[Connection]) -> std::io::Result<()> {
    let path = get_config_file_path();
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    for conn in conns {
        let conn = secure_connection_for_storage(conn)?;
        writeln!(file, "{};{};{}", conn.id, conn.uri, conn.name)?;
    }
    save_connection_colors(conns)?;
    Ok(())
}

pub fn delete_connection(id: usize) -> std::io::Result<()> {
    let mut connections = load_connections()?;
    let original_len = connections.len();
    connections.retain(|connection| connection.id != id);
    if connections.len() == original_len {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Connection not found",
        ));
    }
    overwrite_connections(&connections)
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
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;

    let conn = secure_connection_for_storage(&Connection {
        id: new_id,
        uri: uri.to_string(),
        name: name.to_string(),
        color: "#00D084".to_string(),
    })?;

    writeln!(file, "{};{};{}", conn.id, conn.uri, conn.name)?;
    let mut connections = load_connections().unwrap_or_default();
    connections.push(conn);
    save_connection_colors(&connections)?;
    Ok(())
}

pub fn save_connection_with_color(uri: &str, name: &str, color: &str) -> std::io::Result<()> {
    save_connection(uri, name)?;
    let mut connections = load_connections()?;
    if let Some(connection) = connections
        .iter_mut()
        .filter(|connection| connection.name == name)
        .max_by_key(|connection| connection.id)
    {
        connection.color = format!("#{color}", color = color.trim_start_matches('#'));
    }
    overwrite_connections(&connections)
}

pub fn update_connection_with_color(
    id: usize,
    uri: &str,
    name: &str,
    color: &str,
) -> std::io::Result<()> {
    let mut connections = load_connections()?;
    let Some(connection) = connections
        .iter_mut()
        .find(|connection| connection.id == id)
    else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Connection not found",
        ));
    };
    connection.uri = uri.to_string();
    connection.name = name.to_string();
    connection.color = format!("#{color}", color = color.trim_start_matches('#'));
    overwrite_connections(&connections)
}

pub fn parse_connection_color(value: &str) -> Option<ratatui::style::Color> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(ratatui::style::Color::Rgb(red, green, blue))
}

fn save_connection_colors(conns: &[Connection]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(get_connection_colors_path())?;
    for conn in conns {
        writeln!(file, "{};{}", conn.id, conn.color)?;
    }
    Ok(())
}

pub fn resolve_connection_uri(conn: &Connection) -> std::io::Result<String> {
    restore_uri_password(&conn.uri, conn.id)
}

pub fn resolve_connection_uri_by_stored_uri(
    uri: &str,
    conns: &[Connection],
) -> std::io::Result<String> {
    if let Some(conn) = conns.iter().find(|conn| conn.uri == uri) {
        resolve_connection_uri(conn)
    } else {
        Ok(uri.to_string())
    }
}
pub fn load_connections() -> std::io::Result<Vec<Connection>> {
    let path = get_config_file_path();
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        let colors = load_connection_colors();
        let mut connections = Vec::new();

        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split(';').collect();
            if parts.len() == 3 {
                if let Ok(id) = parts[0].parse::<usize>() {
                    connections.push(Connection {
                        id,
                        uri: parts[1].to_string(),
                        name: parts[2].to_string(),
                        color: colors
                            .get(&id)
                            .cloned()
                            .unwrap_or_else(|| "#00D084".to_string()),
                    });
                }
            }
        }

        Ok(connections)
    } else {
        Ok(vec![])
    }
}

fn load_connection_colors() -> std::collections::HashMap<usize, String> {
    let Ok(file) = File::open(get_connection_colors_path()) else {
        return std::collections::HashMap::new();
    };
    BufReader::new(file)
        .lines()
        .flatten()
        .filter_map(|line| {
            let (id, color) = line.split_once(';')?;
            Some((id.parse().ok()?, color.to_string()))
        })
        .collect()
}
pub fn read_clipboard_string() -> Result<String, String> {
    if let Ok(mut cb) = Clipboard::new() {
        if let Ok(text) = cb.get_text() {
            return Ok(text);
        }
    }

    for (program, args) in [
        ("wl-paste", vec!["--no-newline"]),
        ("xclip", vec!["-selection", "clipboard", "-o"]),
        ("xsel", vec!["--clipboard", "--output"]),
    ] {
        if let Ok(output) = Command::new(program).args(args).output()
            && output.status.success()
        {
            return String::from_utf8(output.stdout)
                .map_err(|e| format!("Clipboard text is not valid UTF-8: {e}"));
        }
    }

    Err("Could not read clipboard. Install wl-clipboard, xclip, or xsel.".to_string())
}

#[cfg(target_os = "linux")]
fn write_system_clipboard_string(text: &str) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| format!("Could not open clipboard: {e}"))?;
    let text = text.to_string();
    thread::Builder::new()
        .name("mongodbtui-clipboard".to_string())
        .spawn(move || {
            let _ = cb
                .set()
                .wait_until(Instant::now() + Duration::from_secs(10))
                .text(text);
        })
        .map(|_| ())
        .map_err(|e| format!("Could not start clipboard thread: {e}"))
}

fn write_terminal_clipboard_string(text: &str) -> Result<(), String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    write!(io::stdout(), "\x1b]52;c;{}\x07", encoded)
        .and_then(|_| io::stdout().flush())
        .map_err(|e| format!("Could not write terminal clipboard: {e}"))
}

pub fn write_clipboard_string(text: &str) -> Result<(), String> {
    if std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.contains("kitty"))
    {
        return write_terminal_clipboard_string(text);
    }

    let arboard_result = write_system_clipboard_string(text);
    if arboard_result.is_ok() {
        return Ok(());
    }

    let osc52_result = write_terminal_clipboard_string(text);

    if osc52_result.is_ok() {
        Ok(())
    } else {
        Err(format!(
            "{}; {}",
            arboard_result.unwrap_err(),
            osc52_result.unwrap_err()
        ))
    }
}
