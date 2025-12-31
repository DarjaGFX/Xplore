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
            _ => return Err("Invalid action"),
        }
        Ok(())
    }

    pub fn get_actions(&self) -> Vec<(&'static str, String)> {
        vec![
            ("quit", self.keybindings.quit.clone()),
            ("edit", self.keybindings.edit.clone()),
            ("up", self.keybindings.up.clone()),
            ("down", self.keybindings.down.clone()),
            ("enter", self.keybindings.enter.clone()),
            ("backspace", self.keybindings.backspace.clone()),
            ("settings", self.keybindings.settings.clone()),
            ("search", self.keybindings.search.clone()),
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
        // Shift is usually implicit for uppercase chars, but for special keys it might matter
        // However, for simplicity let's only add if it's not a char
    }

    let code_str = match code {
        crossterm::event::KeyCode::Char(c) => c.to_string(),
        crossterm::event::KeyCode::Enter => "enter".to_string(),
        crossterm::event::KeyCode::Backspace => "backspace".to_string(),
        crossterm::event::KeyCode::Up => "up".to_string(),
        crossterm::event::KeyCode::Down => "down".to_string(),
        crossterm::event::KeyCode::Left => "left".to_string(),
        crossterm::event::KeyCode::Right => "right".to_string(),
        crossterm::event::KeyCode::F(n) => format!("f{}", n),
        crossterm::event::KeyCode::Esc => "esc".to_string(),
        _ => String::new(),
    };

    if code_str.is_empty() {
        return String::new();
    }

    parts.push(&code_str);
    parts.join("+")
}
