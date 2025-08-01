# MongoDB TUI 

A terminal-based TUI application for browsing and managing your MongoDB collections and databases. It supports JSON import/export, document editing, collection management, and script execution.

## Prerequisities

Before installaion make sure following tools are installed : 

### MongoDB

#### Arch Linux (or Arch-based distros)
```sh
yay mongodb-bin
yay mongodb-tools-bin #for import export
```
```sh
sudo systemctl start mongodb
```
```sh
sudo systemctl enable mongodb
```
#### Other Distros
For installation instructions, see:
https://www.mongodb.com/docs/manual/installation/
#### Rust
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
After installation, either restart your terminal or run:
```sh
source $HOME/.cargo/env
```
Then confirm installation with:
```sh
rustc --version
cargo --version
```

### Build & Install
```sh
git clone git@github.com:vehbican/mongodb-tui.git
cd mongodb-tui
chmod +x install.sh
./install.sh
mongodbtui
```
## Keybindings

### Global
| Key        | Action                                |
|------------|----------------------------------------|
| `?`        | Toggle help popup                      |
| `q`        | Quit the application                   |
| `Esc`      | Dismiss popup / Exit insert mode       |

### Focus Navigation
| Key            | Action                           |
|----------------|----------------------------------|
| `Ctrl+l`       | Focus → Filter/Sort              |
| `Ctrl+j`       | Focus → Documents                |
| `Ctrl+k`       | Focus → Filter/Sort              |
| `Ctrl+h`       | Focus → Connections              |

### List Navigation
| Key         | Action                                 |
|-------------|----------------------------------------|
| `j` / `↓`   | Move down                              |
| `k` / `↑`   | Move up                                |
| `Enter`     | Expand item / Load collection / Confirm |

### Connections & Collections
| Key     | Action                                                                 |
|---------|------------------------------------------------------------------------|
| `o`     | Add new MongoDB connection                                             |
| `e`     | Edit selected URI or collection name                                   |
| `x`     | Export selected collection or database                                 |
| `d` + `d` | Delete hovered item:<br>• In Filter: deletes matched documents<br>• In Documents: deletes selected document<br>• In Connections: deletes collection or database |

### Filter & Sort
| Key       | Action                               |
|-----------|--------------------------------------|
| `a`       | Edit filter or sort input            |
| `Tab`     | Toggle between filter and sort input |
| `Enter`   | Apply filter & sort                  |

### Documents
| Key       | Action                               |
|-----------|--------------------------------------|
| `n` / `N` | Navigate fields in document          |
| `e`       | Edit selected field                  |
| `D`       | Delete selected field (except `_id`) |

### Insert Mode
| Key         | Action               |
|-------------|----------------------|
| `Enter`     | Submit input         |
| `Esc`       | Cancel editing       |
| `← / →`     | Move cursor          |
| `Backspace` | Delete character     |

### File Picker (Import / Export / Script)
| Key       | Action                          |
|-----------|---------------------------------|
| `i`       | Import a collection (.json)     |
| `I`       | Import a database (from folder) |
| `f`       | Run a shell script (.sh)        |
| `j / k`   | Navigate entries                |
| `Space`   | Select/Deselect file            |
| `Enter`   | Enter directory                 |
| `c`       | Confirm action (import/run)     |
| `Esc`     | Exit file picker                |

## Config Paths

- The `connections.csv` file is used to store your saved MongoDB connections and is located at:  
  `~/.config/mongodbtui/connections.csv`

- All exported collections and databases (as .json files and folders) are saved under:  
  `~/.local/share/mongodbtui/`
