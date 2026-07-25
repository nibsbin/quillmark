//! Quill file/query convenience methods.
use std::path::{Path, PathBuf};

use super::Quill;

impl Quill {
    /// Get file contents by path (relative to quill root)
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]> {
        self.files.get_file(path)
    }

    /// Check if a file exists in memory
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.file_exists(path)
    }

    /// Check if a directory exists in memory
    pub fn dir_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.dir_exists(path)
    }

    /// List all directories in a directory (returns paths relative to quill root)
    pub fn list_directories<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf> {
        let dir_path = dir_path.as_ref();
        let subdirs = self.files.list_subdirectories(dir_path);

        // Convert subdirectory names to full paths
        subdirs
            .iter()
            .map(|name| {
                if dir_path == Path::new("") {
                    PathBuf::from(name)
                } else {
                    dir_path.join(name)
                }
            })
            .collect()
    }

    /// Get all files matching a pattern (supports glob-style wildcards).
    /// An invalid pattern matches nothing.
    pub fn find_files<P: AsRef<Path>>(&self, pattern: P) -> Vec<PathBuf> {
        let Ok(glob_pattern) = glob::Pattern::new(&pattern.as_ref().to_string_lossy()) else {
            return Vec::new();
        };
        let mut matches = Vec::new();
        // Paths only — the visitor lends the contents, so no bundle bytes are
        // copied to answer a name query.
        self.files.for_each_file(&mut |path, _| {
            if glob_pattern.matches(path) {
                matches.push(PathBuf::from(path));
            }
        });
        matches.sort();
        matches
    }
}
