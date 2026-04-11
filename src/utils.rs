use std::{
    ffi::OsStr,
    fs::{read_dir, read_link, remove_file},
    io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// File-related utilities

pub(crate) fn dir_contents<'a>(
    path: &'a Path,
) -> std::io::Result<impl Iterator<Item = PathBuf> + use<>> {
    let entries = read_dir(path)?;

    Ok(entries.filter_map(|entry| entry.ok().map(|e| e.path())))
}

pub(crate) fn ensure_symlink_exists(symlink_path: &Path, target_path: &Path) -> io::Result<()> {
    // NOTE: Updates are currently non-atomic
    if let Ok(current_target_path_buf) = read_link(symlink_path) {
        if current_target_path_buf.as_path() == target_path {
            return Ok(());
        } else {
            remove_file(symlink_path)?;
        }
    }
    symlink(target_path, symlink_path)
}

pub(crate) fn file_name_as_number(path: &Path) -> Option<u32> {
    path.file_name()
        .and_then(|s| s.parse_int() as Option<u32>)
}

pub(crate) fn file_name_is_number(path: &Path) -> bool {
    file_name_as_number(path).is_some()
}

pub(crate) fn numeric_entries_in_dir(path:&Path) -> Vec<PathBuf> {
    dir_contents(path)
        .map(|paths| paths.filter(|p| file_name_is_number(p)).collect())
        .unwrap_or_else(|_| vec![])
}

pub(crate) fn sorted_numeric_entries_in_dir(path:&Path) -> Vec<PathBuf> {
    let mut numeric_dir_entries = numeric_entries_in_dir(path);
    numeric_dir_entries.sort_by_key(|p| file_name_as_number(p));
    numeric_dir_entries
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

impl<'a> ParseInt<i32> for std::path::Component<'a> {
    fn parse_int(&self) -> Option<i32> {
        self.as_os_str().parse_int()
    }
}

impl<'a> ParseInt<u32> for std::path::Component<'a> {
    fn parse_int(&self) -> Option<u32> {
        self.as_os_str().parse_int()
    }
}
