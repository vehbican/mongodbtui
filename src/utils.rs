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
    thread,
    time::{Duration, Instant},
};

const KEYRING_SERVICE: &str = "mongodbtui";
const PASSWORD_SENTINEL: &str = "__MONGODBTUI_KEYRING__";

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
        });
    }

    let Some((redacted_uri, password)) = redact_uri_password(&conn.uri) else {
        return Ok(Connection {
            id: conn.id,
            uri: conn.uri.clone(),
            name: conn.name.clone(),
        });
    };

    keyring_entry(conn.id)?
        .set_password(&password)
        .map_err(|e| io::Error::other(format!("Could not write keyring password: {e}")))?;

    Ok(Connection {
        id: conn.id,
        uri: redacted_uri,
        name: conn.name.clone(),
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
    connections.retain(|c| c.id != id);

    connections.push(crate::app::Connection { id, uri, name });

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
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;

    let conn = secure_connection_for_storage(&Connection {
        id: new_id,
        uri: uri.to_string(),
        name: name.to_string(),
    })?;

    writeln!(file, "{};{};{}", conn.id, conn.uri, conn.name)?;
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
    let mut cb = Clipboard::new().map_err(|e| format!("Could not open clipboard: {e}"))?;
    cb.get_text()
        .map_err(|e| format!("Could not read clipboard: {e}"))
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
