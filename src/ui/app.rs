use crate::filesystem::{FileSystemManager, FileEntry};
use crate::config::Config;
use crossterm::event::KeyCode;

pub enum InputMode {
    Normal,
    Editing,
    Config,
    Search,
    Remapping(String), // stores the action name being remapped
}

pub struct App {
    pub manager: FileSystemManager,
    pub all_entries: Vec<FileEntry>,
    pub filtered_entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub input_mode: InputMode,
    pub edit_buffer: String,
    pub search_query: String,
    pub config: Config,
    pub config_index: usize,
    pub error_message: Option<String>,
    pub is_searching: bool,
}

impl App {
    pub fn new(path: &str) -> Self {
        let manager = FileSystemManager::new(path);
        let all_entries = manager.list_directory().unwrap_or_default();
        let config = Config::load();
        let filtered_entries = all_entries.clone();
        Self {
            manager,
            all_entries,
            filtered_entries,
            selected_index: 0,
            input_mode: InputMode::Normal,
            edit_buffer: String::new(),
            search_query: String::new(),
            config,
            config_index: 0,
            error_message: None,
            is_searching: false,
        }
    }

    pub fn refresh(&mut self) {
        self.all_entries = self.manager.list_directory().unwrap_or_default();
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_entries = self.all_entries.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_entries = self.all_entries.iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&query) || 
                    e.description.as_ref().map(|d| d.to_lowercase().contains(&query)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        
        if self.selected_index >= self.filtered_entries.len() && !self.filtered_entries.is_empty() {
            self.selected_index = self.filtered_entries.len() - 1;
        } else if self.filtered_entries.is_empty() {
            self.selected_index = 0;
        }
    }

    pub fn on_key(&mut self, code: KeyCode, modifiers: crossterm::event::KeyModifiers) {
        let event_str = crate::config::key_event_to_string(code, modifiers);

        match &self.input_mode {
            InputMode::Normal => {
                if event_str == self.config.keybindings.up || code == KeyCode::Up {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                } else if event_str == self.config.keybindings.down || code == KeyCode::Down {
                    if self.selected_index < self.filtered_entries.len().saturating_sub(1) {
                        self.selected_index += 1;
                    }
                } else if event_str == self.config.keybindings.enter || code == KeyCode::Enter {
                    if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                        if entry.is_dir {
                            let path = entry.path.clone();
                            if self.manager.navigate_to(path).is_ok() {
                                self.search_query.clear(); // Clear search on navigate
                                self.refresh();
                                self.selected_index = 0;
                            }
                        } else {
                            // Try to open file
                            let _ = opener::open(&entry.path);
                        }
                    }
                } else if event_str == self.config.keybindings.backspace || code == KeyCode::Backspace {
                    if self.manager.navigate_up() {
                        self.search_query.clear(); // Clear search on navigate
                        self.refresh();
                        self.selected_index = 0;
                    }
                } else if event_str == self.config.keybindings.edit {
                    if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                        if entry.name != ".." && entry.name != "." {
                            self.edit_buffer = entry.description.clone().unwrap_or_default();
                            self.input_mode = InputMode::Editing;
                        }
                    }
                } else if event_str == self.config.keybindings.settings {
                    self.input_mode = InputMode::Config;
                    self.config_index = 0;
                } else if event_str == self.config.keybindings.search {
                    self.input_mode = InputMode::Search;
                } else if event_str == self.config.keybindings.quit {
                    // Logic handled in main.rs loop usually, but we could set a flag
                } else if code == KeyCode::Esc {
                    self.search_query.clear();
                    self.apply_filter();
                }
            },
            InputMode::Editing => match code {
                KeyCode::F(2) => {
                    // Save on F2
                    if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                        let _ = crate::metadata::set_description(&entry.path, &self.edit_buffer);
                    }
                    self.input_mode = InputMode::Normal;
                    self.refresh();
                }
                KeyCode::Enter => {
                    // Always newline on Enter in multiline editor
                    self.edit_buffer.push('\n');
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                _ => {}
            },
            InputMode::Search => match code {
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                    // Trigger deep search if query is not empty
                    if !self.search_query.is_empty() {
                        self.trigger_deep_search();
                    }
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.apply_filter();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.apply_filter();
                }
                _ => {}
            },
            InputMode::Config => match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.config_index > 0 {
                        self.config_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.config_index < self.config.get_actions().len().saturating_sub(1) {
                        self.config_index += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some((action, _)) = self.config.get_actions().get(self.config_index) {
                        self.input_mode = InputMode::Remapping(action.to_string());
                        self.error_message = None;
                    }
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.error_message = None;
                }
                _ => {}
            },
            InputMode::Remapping(action) => {
                if !event_str.is_empty() && code != KeyCode::Esc {
                    let action_clone = action.clone();
                    match self.config.set_key(&action_clone, event_str) {
                        Ok(_) => {
                            let _ = self.config.save();
                            self.input_mode = InputMode::Config;
                            self.error_message = None;
                        }
                        Err(e) => {
                            self.error_message = Some(e.to_string());
                        }
                    }
                } else if code == KeyCode::Esc {
                    self.input_mode = InputMode::Config;
                    self.error_message = None;
                }
            }
        }
    }

    pub fn trigger_deep_search(&mut self) {
        self.is_searching = true;
        // Start deep search from root "/" instead of current path to be "Global"
        let root = if cfg!(windows) { "C:\\" } else { "/" };
        let results = self.manager.search_recursive(root, &self.search_query);
        self.filtered_entries = results;
        self.is_searching = false;
        self.selected_index = 0;
    }
}
