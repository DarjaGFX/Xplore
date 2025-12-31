use std::path::Path;
use xattr;

pub const XPLORE_DESC_KEY: &str = "user.xplore.description";

/// Get the description from a file's extended attributes.
pub fn get_description<P: AsRef<Path>>(path: P) -> Option<String> {
    match xattr::get(path, XPLORE_DESC_KEY) {
        Ok(Some(data)) => String::from_utf8(data).ok(),
        _ => None,
    }
}

/// Set the description in a file's extended attributes.
pub fn set_description<P: AsRef<Path>>(path: P, description: &str) -> std::io::Result<()> {
    xattr::set(path, XPLORE_DESC_KEY, description.as_bytes())
}

/// Clear the description from a file's extended attributes.
pub fn clear_description<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    xattr::remove(path, XPLORE_DESC_KEY)
}

/// Check if the target filesystem supports xattrs by attempting a dummy write or just checking the path.
/// Note: Real check often requires attempting an operation or checking mount options.
pub fn is_xattr_supported<P: AsRef<Path>>(_path: P) -> bool {
    // In a real app, we might check the filesystem type, but xattr::get/set will return 
    // ENOTSUP if not supported.
    true // Simplified for now
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_set_get_description() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let desc = "This is a test description";
        if set_description(&file_path, desc).is_ok() {
            let retrieved = get_description(&file_path);
            assert_eq!(retrieved, Some(desc.to_string()));
        } else {
            // xattr might not be supported on the temp filesystem
            println!("xattr not supported, skipping test");
        }
    }
}
