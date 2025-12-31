use crate::filesystem::{FileSystemManager, FileEntry};
use crate::config::Config;
use crossterm::event::KeyCode;

use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone)]
pub enum PromptType {
    NewFolder,
    DeleteConfirmation,
}

#[derive(Clone)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone)]
pub struct Clipboard {
    pub paths: HashSet<PathBuf>,
    pub mode: ClipboardMode,
}

pub enum InputMode {
    Normal,
    Editing,
    Search,
    Config,
    Remapping(String),
    Prompt(PromptType),
    Help,
}

use ratatui::widgets::ListState;

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
    pub selected_paths: HashSet<PathBuf>,
    pub clipboard: Option<Clipboard>,
    pub prompt_buffer: String,
    pub list_state: ListState,
    pub list_height: u16,
}

impl App {
    pub fn new() -> Self {
        let manager = FileSystemManager::new(".");
        let config = Config::load();
        let mut app = Self {
            manager,
            all_entries: Vec::new(),
            filtered_entries: Vec::new(),
            selected_index: 0,
            input_mode: InputMode::Normal,
            edit_buffer: String::new(),
            search_query: String::new(),
            config,
            config_index: 0,
            error_message: None,
            is_searching: false,
            selected_paths: HashSet::new(),
            clipboard: None,
            prompt_buffer: String::new(),
            list_state: ListState::default(),
            list_height: 0,
        };
        app.refresh();
        app
    }

    pub fn is_selected(&self, path: &PathBuf) -> bool {
        self.selected_paths.contains(path)
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
        self.list_state.select(Some(self.selected_index));
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
                                self.clear_selection_if_needed();
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
                        self.clear_selection_if_needed();
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
                } else if event_str == self.config.keybindings.ctrl_home {
                    self.selected_index = 0;
                } else if event_str == self.config.keybindings.ctrl_end {
                    self.selected_index = self.filtered_entries.len().saturating_sub(1);
                } else if event_str == self.config.keybindings.home || code == KeyCode::Home {
                    self.selected_index = self.list_state.offset();
                } else if event_str == self.config.keybindings.end || code == KeyCode::End {
                    let offset = self.list_state.offset();
                    let height = self.list_height as usize;
                    self.selected_index = (offset + height).saturating_sub(1).min(self.filtered_entries.len().saturating_sub(1));
                } else if event_str == self.config.keybindings.page_up || code == KeyCode::PageUp {
                    let height = self.list_height as usize;
                    self.selected_index = self.selected_index.saturating_sub(height);
                } else if event_str == self.config.keybindings.page_down || code == KeyCode::PageDown {
                    let height = self.list_height as usize;
                    self.selected_index = (self.selected_index + height).min(self.filtered_entries.len().saturating_sub(1));
                } else if event_str == self.config.keybindings.select_all {
                    for entry in &self.filtered_entries {
                        if entry.name != ".." && entry.name != "." {
                            self.selected_paths.insert(entry.path.clone());
                        }
                    }
                } else if event_str == self.config.keybindings.deselect_all {
                    self.selected_paths.clear();
                } else if event_str == self.config.keybindings.select {
                    if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                        if entry.name != ".." && entry.name != "." {
                            if self.selected_paths.contains(&entry.path) {
                                self.selected_paths.remove(&entry.path);
                            } else {
                                self.selected_paths.insert(entry.path.clone());
                            }
                        }
                    }
                } else if event_str == self.config.keybindings.copy {
                    self.perform_clipboard_action(ClipboardMode::Copy);
                } else if event_str == self.config.keybindings.cut {
                    self.perform_clipboard_action(ClipboardMode::Cut);
                } else if event_str == self.config.keybindings.paste {
                    self.paste_clipboard();
                } else if event_str == self.config.keybindings.new_folder {
                    self.prompt_buffer.clear();
                    self.input_mode = InputMode::Prompt(PromptType::NewFolder);
                } else if event_str == self.config.keybindings.delete {
                    self.input_mode = InputMode::Prompt(PromptType::DeleteConfirmation);
                } else if event_str == self.config.keybindings.help {
                    self.input_mode = InputMode::Help;
                } else if event_str == self.config.keybindings.quit {
                    // Logic handled in main.rs loop
                } else if code == KeyCode::Esc {
                    self.selected_paths.clear();
                    self.search_query.clear();
                    self.apply_filter();
                }
                self.list_state.select(Some(self.selected_index));
            },
            InputMode::Prompt(prompt_type) => match code {
                KeyCode::Enter => {
                    match prompt_type {
                        PromptType::NewFolder => {
                            let path = self.manager.current_path().join(&self.prompt_buffer);
                            let _ = self.manager.create_dir(&path);
                        }
                        PromptType::DeleteConfirmation => {
                            if self.prompt_buffer.to_lowercase() == "y" {
                                self.delete_selected();
                            }
                        }
                    }
                    self.input_mode = InputMode::Normal;
                    self.prompt_buffer.clear();
                    self.refresh();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.prompt_buffer.clear();
                }
                KeyCode::Char(c) => {
                    self.prompt_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.prompt_buffer.pop();
                }
                _ => {}
            },
            InputMode::Help => {
                if code == KeyCode::Esc || code == KeyCode::F(1) || event_str == self.config.keybindings.help {
                    self.input_mode = InputMode::Normal;
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

    fn perform_clipboard_action(&mut self, mode: ClipboardMode) {
        let mut paths = self.selected_paths.clone();
        if paths.is_empty() {
            if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                if entry.name != ".." && entry.name != "." {
                    paths.insert(entry.path.clone());
                }
            }
        }
        if !paths.is_empty() {
            self.clipboard = Some(Clipboard { paths, mode });
            self.selected_paths.clear();
        }
    }

    fn paste_clipboard(&mut self) {
        if let Some(clipboard) = self.clipboard.clone() {
            for src in clipboard.paths {
                if let Some(file_name) = src.file_name() {
                    let dst = self.manager.current_path().join(file_name);
                    match clipboard.mode {
                        ClipboardMode::Copy => {
                            let _ = self.manager.copy_recursive(&src, &dst);
                        }
                        ClipboardMode::Cut => {
                            let _ = self.manager.move_entry(&src, &dst);
                        }
                    }
                }
            }
            if let ClipboardMode::Cut = clipboard.mode {
                self.clipboard = None;
            }
            self.refresh();
        }
    }

    fn delete_selected(&mut self) {
        let mut paths = self.selected_paths.clone();
        if paths.is_empty() {
            if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                if entry.name != ".." && entry.name != "." {
                    paths.insert(entry.path.clone());
                }
            }
        }
        for path in paths {
            let _ = self.manager.delete_recursive(&path);
        }
        self.selected_paths.clear();
        self.refresh();
    }

    fn clear_selection_if_needed(&mut self) {
        // Only clear if the selected items are NOT in the clipboard.
        // If they ARE in the clipboard, the user might want to navigate to paste them.
        let mut in_clipboard = false;
        if let Some(clipboard) = &self.clipboard {
            for path in &self.selected_paths {
                if clipboard.paths.contains(path) {
                    in_clipboard = true;
                    break;
                }
            }
        }
        
        if !in_clipboard {
            self.selected_paths.clear();
        }
    }
}
