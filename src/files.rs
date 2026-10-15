// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains utility functions for file operations

use std::fs;
use std::path::Path;

pub const FNAME_MAX: usize = 256;
pub const FENTRIES_MAX: usize = 256;
pub const FILEPATH_MAX: usize = 512;

pub const PARENT_DIR: &str = "..";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
}

/// List of sub-directories and swu-files in a directory, both sorted alphabetically.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirInfo {
    pub dirs: Vec<String>,
    pub swu_files: Vec<FileInfo>,
}

impl DirInfo {
    pub fn entry_count(&self) -> usize {
        self.dirs.len() + self.swu_files.len()
    }

    /// Keep at most `max` entries, directories first.
    pub fn truncate(&mut self, max: usize) {
        self.dirs.truncate(max);
        self.swu_files.truncate(max - self.dirs.len());
    }

    /// Remove ".." when the current directory is the top directory, which may not be left.
    pub fn remove_parent_entry(&mut self, top_dir: &str, current_dir: &str) {
        if top_dir == current_dir {
            self.dirs.retain(|d| d != PARENT_DIR);
        }
    }
}

/// Get filename extension; names without a dot or starting with the only dot have none.
pub fn extension(filename: &str) -> &str {
    match filename.rfind('.') {
        None | Some(0) => "",
        Some(dot) => &filename[dot + 1..],
    }
}

/// List all "swu" files & sub-directories in a directory. The parent entry ("..") is
/// always the first directory; the current directory (".") is not listed.
pub fn list_all_swu_files(source_dir: &Path) -> DirInfo {
    let mut info = DirInfo::default();

    let entries = match fs::read_dir(source_dir) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("Unable to read directory {}: {err}", source_dir.display());
            return info;
        }
    };

    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            log::warn!("Skipping entry with non UTF-8 name: {:?}", entry.file_name());
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            info.dirs.push(name);
        } else if file_type.is_file() && extension(&name) == "swu" {
            let size = entry.metadata().map(|m| m.len()).unwrap_or_else(|err| {
                log::error!("Error getting file-size for {name}: {err}");
                0
            });
            info.swu_files.push(FileInfo { name, size });
        }
    }

    info.dirs.sort();
    info.dirs.insert(0, PARENT_DIR.to_string());
    info.swu_files.sort_by(|a, b| a.name.cmp(&b.name));

    info
}

/// If the selected dir is "..", move one level up, else move one level down.
///
/// Returns false if the resulting path is not allowed.
pub fn update_current_dir(current_dir: &mut String, selected_dir: &str) -> bool {
    if selected_dir == PARENT_DIR {
        match current_dir.rfind('/') {
            None | Some(0) => {
                log::warn!("Invalid top dir: {current_dir}");
                false
            }
            Some(slash) => {
                current_dir.truncate(slash);
                true
            }
        }
    } else {
        if current_dir.len() + selected_dir.len() + 2 > FNAME_MAX {
            return false;
        }
        current_dir.push('/');
        current_dir.push_str(selected_dir);
        true
    }
}

/// Create label-text for file-entry in file-browser, e.g. "filename (filesize B)"
pub fn file_entry_label(file_name: &str, file_size: u64) -> Option<String> {
    let label = format!("{file_name} ({file_size} B)");
    (label.len() < FILEPATH_MAX).then_some(label)
}

/// Check for directory-name length
pub fn is_valid_dir_name_length(dir_name: &str) -> bool {
    dir_name.len() < FILEPATH_MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "recovery-gui-test-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn file(&self, name: &str, content: &[u8]) {
            fs::write(self.0.join(name), content).unwrap();
        }

        fn dir(&self, name: &str) {
            fs::create_dir(self.0.join(name)).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn file_size_and_listing() {
        let tmp = TempDir::new("listing");
        tmp.file("hello.swu", b"12345");
        tmp.file("tmpabx10.swu", b"");
        tmp.file("notes.txt", b"ignored");
        tmp.file(".swu", b"no extension");
        tmp.dir("dirB");
        tmp.dir("dirA");

        let info = list_all_swu_files(&tmp.0);

        assert_eq!(info.dirs, ["..", "dirA", "dirB"]);
        assert_eq!(
            info.swu_files,
            [
                FileInfo { name: "hello.swu".into(), size: 5 },
                FileInfo { name: "tmpabx10.swu".into(), size: 0 },
            ]
        );
        assert_eq!(info.entry_count(), 5);
    }

    #[test]
    fn listing_of_missing_dir_is_empty() {
        let info = list_all_swu_files(Path::new("/nonexistent/recovery-gui"));
        assert_eq!(info.entry_count(), 0);
    }

    #[test]
    fn symlinks_are_not_followed() {
        let tmp = TempDir::new("links");
        tmp.file("real.swu", b"x");
        std::os::unix::fs::symlink(tmp.0.join("real.swu"), tmp.0.join("link.swu")).unwrap();

        let info = list_all_swu_files(&tmp.0);

        assert_eq!(info.swu_files.len(), 1);
        assert_eq!(info.swu_files[0].name, "real.swu");
    }

    #[test]
    fn get_extension() {
        assert_eq!(extension("./abc.txt"), "txt");
        assert_eq!(extension("./abc.swu.txt"), "txt");
        assert_eq!(extension("./abc.swu"), "swu");
        assert_eq!(extension("./abc.txt.swu"), "swu");
        assert_eq!(extension("."), "");
        assert_eq!(extension(".."), "");
        assert_eq!(extension(".././"), "/");
        assert_eq!(extension("../.  "), "  ");
        assert_eq!(extension(".swu"), "");
    }

    #[test]
    fn remove_parent_entry() {
        let mut info = DirInfo {
            dirs: vec!["..".into(), "dir1".into()],
            swu_files: vec![],
        };

        info.remove_parent_entry("/media", "/media/usb");
        assert_eq!(info.dirs, ["..", "dir1"]);

        info.remove_parent_entry("/media/usb", "/media/usb");
        assert_eq!(info.dirs, ["dir1"]);
    }

    #[test]
    fn truncate_entries() {
        let mut info = DirInfo {
            dirs: vec!["..".into(), "a".into()],
            swu_files: vec![
                FileInfo { name: "1.swu".into(), size: 0 },
                FileInfo { name: "2.swu".into(), size: 0 },
            ],
        };
        info.truncate(3);
        assert_eq!(info.entry_count(), 3);
        assert_eq!(info.swu_files.len(), 1);
    }

    #[test]
    fn update_current_dir_up_and_down() {
        let mut current = String::from("/media/usb/sda");
        assert!(update_current_dir(&mut current, ".."));
        assert_eq!(current, "/media/usb");

        assert!(update_current_dir(&mut current, ".."));
        assert_eq!(current, "/media");

        assert!(!update_current_dir(&mut current, ".."));
        assert_eq!(current, "/media");

        assert!(update_current_dir(&mut current, "user1"));
        assert_eq!(current, "/media/user1");

        assert!(update_current_dir(&mut current, "usb_disk"));
        assert_eq!(current, "/media/user1/usb_disk");

        let mut long = String::from("/media");
        assert!(!update_current_dir(&mut long, &"x".repeat(FNAME_MAX)));
        assert_eq!(long, "/media");
    }

    #[test]
    fn create_file_entry_label() {
        assert_eq!(file_entry_label("ABCD", 14).as_deref(), Some("ABCD (14 B)"));
        assert_eq!(file_entry_label(&"x".repeat(FILEPATH_MAX - 1), 14), None);
    }

    #[test]
    fn dir_name_length() {
        assert!(is_valid_dir_name_length(&"x".repeat(10)));
        assert!(is_valid_dir_name_length(&"x".repeat(FILEPATH_MAX - 1)));
        assert!(!is_valid_dir_name_length(&"x".repeat(FILEPATH_MAX + 1)));
    }
}
