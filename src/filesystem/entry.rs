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
    pub permissions: String,
    pub owner: String,
    pub group: String,
}

impl FileEntry {
    pub fn human_size(&self) -> String {
        if self.is_dir {
            return "---".to_string();
        }
        let size = self.size as f64;
        let units = ["B", "KB", "MB", "GB", "TB", "PB"];
        let mut i = 0;
        let mut val = size;
        while val >= 1024.0 && i < units.len() - 1 {
            val /= 1024.0;
            i += 1;
        }
        if i == 0 {
            format!("{} {}", val as u64, units[i])
        } else {
            format!("{:.2} {}", val, units[i])
        }
    }
}
