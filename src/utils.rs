use std::{ffi::OsStr, fs::{read_dir, read_link, remove_file}, os::unix::fs::symlink, path::{Path, PathBuf}};

////////////////////////////////////////////////////////////////////////////////////////////////////
// File-related utilities

pub(crate) fn dir_contents(path: &Path) -> std::io::Result<impl Iterator<Item = PathBuf>> {
    let entries = read_dir(path)?;

    Ok(entries.filter_map(|entry| {
        entry.ok().map(|e| e.path())
    }))
}

pub(crate) fn ensure_symlink(symlink_path: &Path, target_path: &Path) {
    // Updates are currently non-atomic
    if let Ok(current_target_path_buf) = read_link(symlink_path) {
        if current_target_path_buf.as_path() == target_path {
            return
        } else {
            let _ = remove_file(symlink_path);
        }
    }
    let _ = symlink(target_path, symlink_path);
}

pub(crate) fn file_name_is_number(path: &Path) -> bool {
    path.file_name().and_then(|s| s.parse_int() as Option<u32>).is_some()
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// String parsing uilities

pub(crate) fn is_plus_or_minus_int(s: &str) -> bool {
    (s.starts_with("+") || s.starts_with("-")) && s.parse::<i32>().is_ok()
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ParseInt

pub(crate) trait ParseInt<T> {
    fn parse_int(&self) -> Option<T>;
}

impl ParseInt<i32> for OsStr {
    fn parse_int(&self) -> Option<i32> {
        self.to_str().and_then(|s| s.parse::<i32>().ok())
    }
}

impl ParseInt<u32> for OsStr {
    fn parse_int(&self) -> Option<u32> {
        self.to_str().and_then(|s| s.parse::<u32>().ok())
    }
}
