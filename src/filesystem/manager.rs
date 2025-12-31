use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use crate::filesystem::FileEntry;
use crate::metadata;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
        let meta_dot = fs::metadata(&self.current_dir)?;
        let (perm_dot, owner_dot, group_dot) = self.get_metadata_info(&meta_dot);
        entries.push(FileEntry {
            name: ".".to_string(),
            path: self.current_dir.clone(),
            size: 0,
            is_dir: true,
            mod_time: meta_dot.modified()?.into(),
            description: None,
            permissions: perm_dot,
            owner: owner_dot,
            group: group_dot,
        });

        // Add ".." entry if not at root
        if let Some(parent) = self.current_dir.parent() {
            let meta_parent = fs::metadata(parent)?;
            let (perm_p, owner_p, group_p) = self.get_metadata_info(&meta_parent);
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                size: 0,
                is_dir: true,
                mod_time: meta_parent.modified()?.into(),
                description: None,
                permissions: perm_p,
                owner: owner_p,
                group: group_p,
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
            let (permissions, owner, group) = self.get_metadata_info(&metadata);

            entries.push(FileEntry {
                name,
                path,
                size,
                is_dir,
                mod_time,
                description,
                permissions,
                owner,
                group,
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
            self.copy_recursive(src, dst)?;
            
            // Re-apply xattrs to destination
            for (attr, val) in attr_data {
                let _ = xattr::set(dst, attr, &val);
            }
            
            self.delete_recursive(src)?;
        } else {
            // Rename might have stripped xattrs depending on the OS/FS, 
            // but usually it preserves them if within the same FS.
            // To be safe, we can try to re-apply if they are missing.
            for (attr, val) in attr_data {
                let _ = xattr::set(dst, attr, &val);
            }
        }

        Ok(())
    }

    pub fn create_dir(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub fn delete_recursive(&self, path: &Path) -> std::io::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    }

    pub fn copy_recursive(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        if src.is_dir() {
            std::fs::create_dir_all(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let file_name = entry.file_name();
                self.copy_recursive(&src.join(&file_name), &dst.join(&file_name))?;
            }
        } else {
            std::fs::copy(src, dst)?;
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
                    let (permissions, owner, group) = self.get_metadata_info(&metadata);
                    Some(FileEntry {
                        name,
                        path: path.to_path_buf(),
                        size: metadata.len(),
                        is_dir: metadata.is_dir(),
                        mod_time: metadata.modified().ok()?.into(),
                        description,
                        permissions,
                        owner,
                        group,
                    })
                } else {
                    None
                }
            })
            .take(1000) // Limit results for performance
            .collect()
    }

    fn get_metadata_info(&self, metadata: &fs::Metadata) -> (String, String, String) {
        #[cfg(unix)]
        {
            let mode = metadata.permissions().mode();
            let permissions = format!(
                "{}{}{}{}{}{}{}{}{}{}",
                if metadata.is_dir() { "d" } else { "-" },
                if mode & 0o400 != 0 { "r" } else { "-" },
                if mode & 0o200 != 0 { "w" } else { "-" },
                if mode & 0o100 != 0 { "x" } else { "-" },
                if mode & 0o040 != 0 { "r" } else { "-" },
                if mode & 0o020 != 0 { "w" } else { "-" },
                if mode & 0o010 != 0 { "x" } else { "-" },
                if mode & 0o004 != 0 { "r" } else { "-" },
                if mode & 0o002 != 0 { "w" } else { "-" },
                if mode & 0o001 != 0 { "x" } else { "-" },
            );
            
            let owner = metadata.uid().to_string();
            let group = metadata.gid().to_string();
            (permissions, owner, group)
        }
        #[cfg(not(unix))]
        {
            ("-".to_string(), "unknown".to_string(), "unknown".to_string())
        }
    }
}
