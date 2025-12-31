use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use crate::filesystem::FileEntry;
use crate::metadata;

pub struct FileSystemManager {
    current_dir: PathBuf,
}

impl FileSystemManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let abs_path = fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from("."));
        Self {
            current_dir: abs_path,
        }
    }

    pub fn current_path(&self) -> &Path {
        &self.current_dir
    }

    pub fn list_directory(&self) -> std::io::Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        // Add "." entry
        entries.push(FileEntry {
            name: ".".to_string(),
            path: self.current_dir.clone(),
            size: 0,
            is_dir: true,
            mod_time: fs::metadata(&self.current_dir)?.modified()?.into(),
            description: None,
        });

        // Add ".." entry if not at root
        if let Some(parent) = self.current_dir.parent() {
            let metadata = fs::metadata(parent)?;
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                size: 0,
                is_dir: true,
                mod_time: metadata.modified()?.into(),
                description: None,
            });
        }

        for entry in fs::read_dir(&self.current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let size = metadata.len();
            let mod_time: DateTime<Local> = metadata.modified()?.into();
            let name = entry.file_name().to_string_lossy().to_string();
            
            let description = metadata::get_description(&path);

            entries.push(FileEntry {
                name,
                path,
                size,
                is_dir,
                mod_time,
                description,
            });
        }
        
        // Sort: "." first, then "..", then directories, then alphabetically
        entries.sort_by(|a, b| {
            if a.name == "." {
                std::cmp::Ordering::Less
            } else if b.name == "." {
                std::cmp::Ordering::Greater
            } else if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else if a.is_dir == b.is_dir {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            } else {
                b.is_dir.cmp(&a.is_dir)
            }
        });

        Ok(entries)
    }

    pub fn navigate_to<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let full_path = self.current_dir.join(path);
        let new_path = fs::canonicalize(&full_path).unwrap_or(full_path);
        
        if new_path.is_dir() {
            self.current_dir = new_path;
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotADirectory, "Not a directory"))
        }
    }

    pub fn navigate_up(&mut self) -> bool {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            true
        } else {
            false
        }
    }

    /// Custom move that preserves xattrs even across partitions
    pub fn move_entry(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        // 1. Read metadata (xattrs) from source
        let mut attr_data = Vec::new();
        if let Ok(attrs) = xattr::list(src) {
            for attr in attrs {
                if let Ok(Some(val)) = xattr::get(src, &attr) {
                    attr_data.push((attr, val));
                }
            }
        }

        // 2. Attempt standard rename
        if fs::rename(src, dst).is_err() {
            // 3. Fallback: Copy and Delete (Cross-device move)
            fs::copy(src, dst)?;
            
            // Re-apply xattrs to destination
            for (attr, val) in attr_data {
                xattr::set(dst, attr, &val)?;
            }
            
            fs::remove_file(src)?;
        } else {
            // Rename might have stripped xattrs depending on the OS/FS, 
            // but usually it preserves them if within the same FS.
            // To be safe, we can try to re-apply if they are missing,
            // but rename is usually atomic and preserves metadata.
        }

        Ok(())
    }

    pub fn search_recursive<P: AsRef<Path>>(&self, root: P, query: &str) -> Vec<FileEntry> {
        let query = query.to_lowercase();
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                
                // Only include if name or description matches
                let description = metadata::get_description(path);
                let matches = name.to_lowercase().contains(&query) || 
                             description.as_ref().map(|d| d.to_lowercase().contains(&query)).unwrap_or(false);

                if matches {
                    let metadata = entry.metadata().ok()?;
                    Some(FileEntry {
                        name,
                        path: path.to_path_buf(),
                        size: metadata.len(),
                        is_dir: metadata.is_dir(),
                        mod_time: metadata.modified().ok()?.into(),
                        description,
                    })
                } else {
                    None
                }
            })
            .take(1000) // Limit results for performance
            .collect()
    }
}
