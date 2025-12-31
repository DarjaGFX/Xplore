use chrono::{DateTime, Local};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub mod_time: DateTime<Local>,
    pub description: Option<String>,
}
