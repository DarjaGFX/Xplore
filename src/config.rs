use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: Keybindings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keybindings {
    pub quit: String,
    pub edit: String,
    pub up: String,
    pub down: String,
    pub enter: String,
    pub backspace: String,
    pub settings: String,
    pub search: String,
    pub select: String,
    pub copy: String,
    pub cut: String,
    pub paste: String,
    pub new_folder: String,
    pub delete: String,
    pub help: String,
    pub home: String,
    pub end: String,
    pub ctrl_home: String,
    pub ctrl_end: String,
    pub page_up: String,
    pub page_down: String,
    pub select_all: String,
    pub deselect_all: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybindings: Keybindings {
                quit: "q".to_string(),
                edit: "e".to_string(),
                up: "k".to_string(),
                down: "j".to_string(),
                enter: "enter".to_string(),
                backspace: "backspace".to_string(),
                settings: "s".to_string(),
                search: "f3".to_string(),
                select: "space".to_string(),
                copy: "ctrl+c".to_string(),
                cut: "ctrl+x".to_string(),
                paste: "ctrl+v".to_string(),
                new_folder: "ctrl+n".to_string(),
                delete: "shift+delete".to_string(),
                help: "f1".to_string(),
                home: "home".to_string(),
                end: "end".to_string(),
                ctrl_home: "ctrl+home".to_string(),
                ctrl_end: "ctrl+end".to_string(),
                page_up: "pageup".to_string(),
                page_down: "pagedown".to_string(),
                select_all: "ctrl+a".to_string(),
                deselect_all: "ctrl+d".to_string(),
            },
        }
    }
}

impl Config {
    pub fn get_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("xplore/config.toml")
    }

    pub fn load() -> Self {
        let config_path = Self::get_path();
        let config_dir = config_path.parent().unwrap();

        if !config_path.exists() {
            let _ = fs::create_dir_all(config_dir);
            let default_config = Self::default();
            let _ = default_config.save();
            return default_config;
        }

        if let Ok(content) = fs::read_to_string(config_path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::get_path();
        let toml = toml::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(config_path, toml)
    }

    pub fn is_key_taken(&self, key: &str, exclude_action: &str) -> bool {
        for (action, current_key) in self.get_actions() {
            if action != exclude_action && current_key == key {
                return true;
            }
        }
        false
    }

    pub fn set_key(&mut self, action: &str, key: String) -> Result<(), &'static str> {
        if self.is_key_taken(&key, action) {
            return Err("Key already assigned to another action");
        }
        match action {
            "quit" => self.keybindings.quit = key,
            "edit" => self.keybindings.edit = key,
            "up" => self.keybindings.up = key,
            "down" => self.keybindings.down = key,
            "enter" => self.keybindings.enter = key,
            "backspace" => self.keybindings.backspace = key,
            "settings" => self.keybindings.settings = key,
            "search" => self.keybindings.search = key,
            "select" => self.keybindings.select = key,
            "copy" => self.keybindings.copy = key,
            "cut" => self.keybindings.cut = key,
            "paste" => self.keybindings.paste = key,
            "new_folder" => self.keybindings.new_folder = key,
            "delete" => self.keybindings.delete = key,
            "help" => self.keybindings.help = key,
            "home" => self.keybindings.home = key,
            "end" => self.keybindings.end = key,
            "ctrl_home" => self.keybindings.ctrl_home = key,
            "ctrl_end" => self.keybindings.ctrl_end = key,
            "page_up" => self.keybindings.page_up = key,
            "page_down" => self.keybindings.page_down = key,
            "select_all" => self.keybindings.select_all = key,
            "deselect_all" => self.keybindings.deselect_all = key,
            _ => return Err("Invalid action"),
        }
        Ok(())
    }

    pub fn get_actions(&self) -> Vec<(&'static str, String)> {
        vec![
            // Navigation
            ("up", self.keybindings.up.clone()),
            ("down", self.keybindings.down.clone()),
            ("enter", self.keybindings.enter.clone()),
            ("backspace", self.keybindings.backspace.clone()),
            ("help", self.keybindings.help.clone()),
            ("quit", self.keybindings.quit.clone()),
            ("settings", self.keybindings.settings.clone()),
            // Advanced Navigation
            ("home", self.keybindings.home.clone()),
            ("end", self.keybindings.end.clone()),
            ("ctrl_home", self.keybindings.ctrl_home.clone()),
            ("ctrl_end", self.keybindings.ctrl_end.clone()),
            ("page_up", self.keybindings.page_up.clone()),
            ("page_down", self.keybindings.page_down.clone()),
            ("select_all", self.keybindings.select_all.clone()),
            ("deselect_all", self.keybindings.deselect_all.clone()),
            // File Operations
            ("select", self.keybindings.select.clone()),
            ("copy", self.keybindings.copy.clone()),
            ("cut", self.keybindings.cut.clone()),
            ("paste", self.keybindings.paste.clone()),
            ("new_folder", self.keybindings.new_folder.clone()),
            ("delete", self.keybindings.delete.clone()),
            ("edit", self.keybindings.edit.clone()),
            // Search
            ("search", self.keybindings.search.clone()),
        ]
    }

    pub fn get_categorized_actions(&self) -> Vec<(&'static str, Vec<(&'static str, String)>)> {
        vec![
            ("Navigation", vec![
                ("up", self.keybindings.up.clone()),
                ("down", self.keybindings.down.clone()),
                ("enter", self.keybindings.enter.clone()),
                ("backspace", self.keybindings.backspace.clone()),
                ("help", self.keybindings.help.clone()),
                ("quit", self.keybindings.quit.clone()),
                ("settings", self.keybindings.settings.clone()),
            ]),
            ("Advanced Navigation", vec![
                ("home", self.keybindings.home.clone()),
                ("end", self.keybindings.end.clone()),
                ("ctrl_home", self.keybindings.ctrl_home.clone()),
                ("ctrl_end", self.keybindings.ctrl_end.clone()),
                ("page_up", self.keybindings.page_up.clone()),
                ("page_down", self.keybindings.page_down.clone()),
            ]),
            ("File Operations", vec![
                ("select", self.keybindings.select.clone()),
                ("select_all", self.keybindings.select_all.clone()),
                ("deselect_all", self.keybindings.deselect_all.clone()),
                ("copy", self.keybindings.copy.clone()),
                ("cut", self.keybindings.cut.clone()),
                ("paste", self.keybindings.paste.clone()),
                ("new_folder", self.keybindings.new_folder.clone()),
                ("delete", self.keybindings.delete.clone()),
                ("edit", self.keybindings.edit.clone()),
            ]),
            ("Search", vec![
                ("search", self.keybindings.search.clone()),
            ]),
        ]
    }

    pub fn get_hint(&self, action: &str) -> String {
        match action {
            "quit" => format!("[{}] Quit", self.keybindings.quit),
            "edit" => format!("[{}] Edit", self.keybindings.edit),
            "up" => format!("[{}] Up", self.keybindings.up),
            "down" => format!("[{}] Down", self.keybindings.down),
            "enter" => format!("[{}] Open", self.keybindings.enter),
            "backspace" => format!("[{}] Parent", self.keybindings.backspace),
            "settings" => format!("[{}] Settings", self.keybindings.settings),
            "search" => format!("[{}] Search", self.keybindings.search),
            "select" => format!("[{}] Select", self.keybindings.select),
            "copy" => format!("[{}] Copy", self.keybindings.copy),
            "cut" => format!("[{}] Cut", self.keybindings.cut),
            "paste" => format!("[{}] Paste", self.keybindings.paste),
            "new_folder" => format!("[{}] New Folder", self.keybindings.new_folder),
            "delete" => format!("[{}] Delete", self.keybindings.delete),
            "help" => format!("[{}] Help", self.keybindings.help),
            "home" => format!("[{}] Visible Top", self.keybindings.home),
            "end" => format!("[{}] Visible Bottom", self.keybindings.end),
            "ctrl_home" => format!("[{}] First Item", self.keybindings.ctrl_home),
            "ctrl_end" => format!("[{}] Last Item", self.keybindings.ctrl_end),
            "page_up" => format!("[{}] Page Up", self.keybindings.page_up),
            "page_down" => format!("[{}] Page Down", self.keybindings.page_down),
            "select_all" => format!("[{}] Select All", self.keybindings.select_all),
            "deselect_all" => format!("[{}] Deselect All", self.keybindings.deselect_all),
            _ => String::new(),
        }
    }
}

pub fn key_event_to_string(code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        parts.push("alt");
    }
    if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
        parts.push("shift");
    }

    let code_str = match code {
        crossterm::event::KeyCode::Char(c) => {
            if c == ' ' {
                "space".to_string()
            } else {
                c.to_lowercase().to_string()
            }
        },
        crossterm::event::KeyCode::Enter => "enter".to_string(),
        crossterm::event::KeyCode::Backspace => "backspace".to_string(),
        crossterm::event::KeyCode::Up => "up".to_string(),
        crossterm::event::KeyCode::Down => "down".to_string(),
        crossterm::event::KeyCode::Left => "left".to_string(),
        crossterm::event::KeyCode::Right => "right".to_string(),
        crossterm::event::KeyCode::F(n) => format!("f{}", n),
        crossterm::event::KeyCode::Esc => "esc".to_string(),
        crossterm::event::KeyCode::Delete => "delete".to_string(),
        crossterm::event::KeyCode::Home => "home".to_string(),
        crossterm::event::KeyCode::End => "end".to_string(),
        crossterm::event::KeyCode::PageUp => "pageup".to_string(),
        crossterm::event::KeyCode::PageDown => "pagedown".to_string(),
        _ => String::new(),
    };

    if code_str.is_empty() {
        return String::new();
    }

    parts.push(&code_str);
    parts.join("+")
}
