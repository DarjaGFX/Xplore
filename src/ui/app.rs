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
use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySize, PtySystem};
use std::sync::mpsc::{channel, Receiver};
use std::io::{Read, Write};
use std::thread;

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
    pub prompt_index: usize,
    // Terminal state (PTY)
    pub is_terminal_open: bool,
    pub terminal_focused: bool,
    pub pty_pair: Option<PtyPair>,
    pub pty_parser: Option<vt100::Parser>,
    pub pty_writer: Option<Box<dyn Write + Send>>,
    pub pty_reader_rx: Option<Receiver<Vec<u8>>>,
    pub shell_id: u32,
    pub last_synced_path: PathBuf,
    pub tick_count: u64,
}

fn find_shell_pid(parent_pid: u32) -> Option<u32> {
    use std::collections::{HashSet, VecDeque};
    
    // 1. Snapshot all processes: PID -> (PPID, Name)
    let Ok(entries) = std::fs::read_dir("/proc") else { return None };
    let mut process_map = std::collections::HashMap::new();
    
    for entry in entries.flatten() {
        let Ok(filename) = entry.file_name().into_string() else { continue };
        let Ok(pid) = filename.parse::<u32>() else { continue };
        
        let path = entry.path();
        // Read stat for PPID
        if let Ok(stat) = std::fs::read_to_string(path.join("stat")) {
            if let Some(r_paren) = stat.rfind(')') {
                let rest = &stat[r_paren+2..];
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() > 1 {
                    if let Ok(ppid) = parts[1].parse::<u32>() {
                        // Read comm for Name
                        let name = if let Ok(comm) = std::fs::read_to_string(path.join("comm")) {
                            comm.trim().to_string()
                        } else {
                            String::new()
                        };
                        process_map.insert(pid, (ppid, name));
                    }
                }
            }
        }
    }

    // 2. Build Adjacency List (PPID -> Children)
    let mut children_map: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for (pid, (ppid, _)) in &process_map {
        children_map.entry(*ppid).or_default().push(*pid);
    }

    // 3. BFS from parent_pid
    let mut queue = VecDeque::new();
    queue.push_back(parent_pid);
    let mut visited = HashSet::new();
    visited.insert(parent_pid);

    while let Some(current_pid) = queue.pop_front() {
        if let Some(children) = children_map.get(&current_pid) {
            for child_pid in children {
                if !visited.contains(child_pid) {
                    visited.insert(*child_pid);
                    
                    // Check if this child is a shell
                    if let Some((_, name)) = process_map.get(child_pid) {
                        let n = name.to_lowercase();
                        if n.ends_with("sh") || n == "fish" || n == "nu" {
                            // Found a shell!
                            return Some(*child_pid);
                        }
                    }
                    
                    // Add to queue to search grandchildren
                    queue.push_back(*child_pid);
                }
            }
        }
    }
    
    None
}

impl App {
    pub fn new() -> Self {
        let manager = FileSystemManager::new(".");
        let config = Config::load();
        
        let current_path = manager.current_path().to_path_buf();

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
            prompt_index: 0,
            // Terminal state
            is_terminal_open: false, 
            terminal_focused: false,
            pty_pair: None,
            pty_parser: None,
            pty_writer: None,
            pty_reader_rx: None,
            shell_id: 0, 
            last_synced_path: current_path,
            tick_count: 0,
        };
        app.refresh();
        app
    }

    pub fn spawn_pty(&mut self) {
        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).expect("Failed to create PTY");
        
        let shell = std::env::var("SHELL").unwrap_or("sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(self.manager.current_path());
        
        let _child = pair.slave.spawn_command(cmd).expect("Failed to spawn shell");
        
        let mut reader = pair.master.try_clone_reader().expect("Failed to clone pty reader");
        let writer = pair.master.take_writer().expect("Failed to take pty writer");
        
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let _ = tx.send(buffer[..n].to_vec());
                    }
                    Ok(_) => break, // EOF
                    Err(_) => break, // Error
                }
            }
        });
        
        let parser = vt100::Parser::new(24, 80, 0);
        
        self.pty_pair = Some(pair);
        self.pty_parser = Some(parser);
        self.pty_writer = Some(writer);
        self.pty_reader_rx = Some(rx);
        self.shell_id = 0; // Reset shell ID to force re-discovery
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

    pub fn tick(&mut self) {
        self.tick_count += 1;

        // Read from PTY
        if let Some(rx) = &self.pty_reader_rx {
             while let Ok(bytes) = rx.try_recv() {
                 if let Some(parser) = &mut self.pty_parser {
                     parser.process(&bytes);
                 }
             }
        }
        
        // 1. Live View Poll (Every ~1s, assuming 100ms tick -> 10 ticks)
        if self.tick_count % 10 == 0 {
             self.refresh(); // Reload files
        }

        // 2. Sync Files -> Terminal
        // If manager path changed since last check (and it wasn't us syncing it), update PTY.
        let current = self.manager.current_path().to_path_buf();
        if current != self.last_synced_path {
             // User navigated in GUI
             // We need to escape the path properly. For now, simple quote.
             let path_str = current.to_string_lossy();
             // Write "cd '<path>'\r"
             if let Some(writer) = &mut self.pty_writer {
                 let cmd = format!("cd '{}'\r", path_str);
                 let _ = writer.write_all(cmd.as_bytes());
                 let _ = writer.flush();
             }
             
             self.last_synced_path = current.clone();
        }
        
        // 3. Sync Terminal -> Files (Linux only)
        if self.shell_id == 0 {
             if self.tick_count % 5 == 0 {
                 if let Some(pid) = find_shell_pid(std::process::id()) {
                      self.shell_id = pid;
                 }
             }
        }
        
        if self.shell_id > 0 && self.tick_count % 5 == 0 {
             match std::fs::read_link(format!("/proc/{}/cwd", self.shell_id)) {
                  Ok(target) => {
                      if target != current {
                           if let Ok(_) = self.manager.navigate_to(target.clone()) {
                                self.last_synced_path = target;
                                self.refresh();
                           }
                      }
                  },
                  Err(_) => {
                      self.shell_id = 0;
                  }
             }
        }
    }

    pub fn on_key(&mut self, code: KeyCode, modifiers: crossterm::event::KeyModifiers) {
        let event_str = crate::config::key_event_to_string(code, modifiers);

        match &self.input_mode {
            InputMode::Normal => {
                if event_str == self.config.keybindings.toggle_terminal {
                    self.is_terminal_open = !self.is_terminal_open;
                    if self.is_terminal_open {
                        self.terminal_focused = true;
                        if self.pty_pair.is_none() {
                             self.spawn_pty();
                        }
                    } else {
                        self.terminal_focused = false;
                    }
                    return;
                }
                if event_str == self.config.keybindings.terminal_prefix {
                    if self.is_terminal_open {
                        self.terminal_focused = !self.terminal_focused;
                    }
                    return;
                }

                if self.terminal_focused {
                    // Check for Ctrl+D (EOF/Close)
                    if modifiers == crossterm::event::KeyModifiers::CONTROL && code == KeyCode::Char('d') {
                        // Close terminal pane and kill PTY
                        self.is_terminal_open = false;
                        self.terminal_focused = false;
                        self.pty_pair = None; // Dropping closes the PTY
                        self.pty_parser = None;
                        self.pty_writer = None;
                        self.pty_reader_rx = None;
                        self.shell_id = 0;
                        return;
                    }
                
                    // Forward input to PTY
                    if let Some(writer) = &mut self.pty_writer {
                        let input_bytes = match code {
                            KeyCode::Char(c) => {
                                if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                    let byte = match c {
                                        'a'..='z' => c as u8 - b'a' + 1,
                                        '[' => 27, // Esc
                                        ' ' => 0, // Null
                                        _ => c as u8, // Fallback
                                    };
                                    vec![byte]
                                } else {
                                    let mut b = [0; 4];
                                    c.encode_utf8(&mut b).as_bytes().to_vec()
                                }
                            }
                            KeyCode::Enter => vec![b'\r'],
                            KeyCode::Backspace => vec![b'\x7f'], 
                            KeyCode::Tab => vec![b'\t'],
                            KeyCode::Esc => vec![b'\x1b'],
                            KeyCode::Up => vec![b'\x1b', b'[', b'A'],
                            KeyCode::Down => vec![b'\x1b', b'[', b'B'],
                            KeyCode::Right => vec![b'\x1b', b'[', b'C'],
                            KeyCode::Left => vec![b'\x1b', b'[', b'D'],
                            KeyCode::PageUp => vec![b'\x1b', b'[', b'5', b'~'],
                            KeyCode::PageDown => vec![b'\x1b', b'[', b'6', b'~'],
                            KeyCode::Delete => vec![b'\x1b', b'[', b'3', b'~'],
                            KeyCode::Home => vec![b'\x1b', b'[', b'H'],
                            KeyCode::End => vec![b'\x1b', b'[', b'F'],
                            _ => vec![],
                        };
                        
                        if !input_bytes.is_empty() {
                             let _ = writer.write_all(&input_bytes);
                             let _ = writer.flush();
                        }
                    }
                    return;
                }

                // Normal file manager keybindings (only when terminal is NOT focused)
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
                                self.search_query.clear();
                                self.refresh();
                                self.selected_index = 0;
                            }
                        } else {
                            let _ = opener::open(&entry.path);
                        }
                    }
                } else if event_str == self.config.keybindings.backspace || code == KeyCode::Backspace {
                    if self.manager.navigate_up() {
                        self.clear_selection_if_needed();
                        self.search_query.clear();
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
                    if !self.selected_paths.is_empty() || self.filtered_entries.get(self.selected_index).is_some() {
                        self.input_mode = InputMode::Prompt(PromptType::DeleteConfirmation);
                        self.prompt_index = 1;
                    }
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
            InputMode::Prompt(prompt_type) => match prompt_type {
                PromptType::NewFolder => match code {
                    KeyCode::Enter => {
                        let name = self.prompt_buffer.clone();
                        if !name.is_empty() {
                            let path = self.manager.current_path().join(name);
                            if let Err(e) = self.manager.create_dir(&path) {
                                self.error_message = Some(e.to_string());
                            } else {
                                self.refresh();
                            }
                        }
                        self.input_mode = InputMode::Normal;
                        self.prompt_buffer.clear();
                    }
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.prompt_buffer.clear();
                    }
                    KeyCode::Char(c) => self.prompt_buffer.push(c),
                    KeyCode::Backspace => {
                        self.prompt_buffer.pop();
                    }
                    _ => {}
                },
                PromptType::DeleteConfirmation => match code {
                    KeyCode::Enter => {
                        if self.prompt_index == 0 {
                            // OK selected
                            self.delete_selected();
                        }
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                        self.prompt_index = 1 - self.prompt_index;
                    }
                    _ => {}
                },
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
