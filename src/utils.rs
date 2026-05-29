use std::{
    ffi::OsStr,
    fs::{read_dir, read_link, remove_file},
    io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// File-related utilities

/// Returns an iterator over the direct contents of a directory.
pub(crate) fn dir_contents<'a>(
    path: &'a Path,
) -> std::io::Result<impl Iterator<Item = PathBuf> + use<>> {
    let entries = read_dir(path)?;

    Ok(entries.filter_map(|entry| entry.ok().map(|e| e.path())))
}

/// Attempts to create/update a symlink of it doesn't already exist with the specified target.
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

/// If the file name component of a path is a string of digits, convert it to a number.
pub(crate) fn file_name_as_number(path: &Path) -> Option<u32> {
    path.file_name()
        .and_then(|s| s.parse_int() as Option<u32>)
}

/// Checks whether the file name component of a path is a number.
pub(crate) fn file_name_is_number(path: &Path) -> bool {
    file_name_as_number(path).is_some()
}

/// Returns the vector of the contents of a directory whose file names are numbers.
/// If the directory is not readable, returns an empty vector rather than an error.
pub(crate) fn numeric_entries_in_dir(path:&Path) -> Vec<PathBuf> {
    dir_contents(path)
        .map(|paths| paths.filter(|p| file_name_is_number(p)).collect())
        .unwrap_or_else(|_| vec![])
}

/// Returns the vector of the contents of a directory whose file names are numbers
/// in ascending order. If the directory is not readable, returns an empty vector
/// rather than an error.
pub(crate) fn sorted_numeric_entries_in_dir(path:&Path) -> Vec<PathBuf> {
    let mut numeric_dir_entries = numeric_entries_in_dir(path);
    numeric_dir_entries.sort_by_key(|p| file_name_as_number(p));
    numeric_dir_entries
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// String parsing uilities

/// Does a string represent a decimal integer with a leading `+` or `-`?
pub(crate) fn is_plus_or_minus_int(s: &str) -> bool {
    (s.starts_with("+") || s.starts_with("-")) && s.parse::<i32>().is_ok()
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ParseInt

/// Implementations of ParseInt provide a parse_int() method on existing types.
/// This is a workaround for the fact that we can't add TryFrom implementations
/// on types declared elsewhere.
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs::File;
    use tempfile::tempdir;

    // Test dir_contents()

    #[test]
    fn test_dir_contents_empty() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let actual_paths = dir_contents(tempdir.path()).expect("dir_contents() did not return an iterator").collect::<HashSet<PathBuf>>();
        assert_eq!(HashSet::new(), actual_paths);
    }

    #[test]
    fn test_dir_contents_nonempty() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let expected_paths = HashSet::from([tempdir.path().join("abc"), tempdir.path().join("xyz"), tempdir.path().join("123")]);
        for path in &expected_paths {
            File::create(path).expect(&format!("failed to create file in tempdir: {}", path.to_str().unwrap()));
        }
        let actual_paths = dir_contents(tempdir.path()).expect("dir_contents() did not return an iterator").collect::<HashSet<PathBuf>>();
        assert_eq!(expected_paths, actual_paths);
    }

    #[test]
    fn test_dir_contents_nonexistent() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        assert!(dir_contents(&tempdir.path().join("does_not_exist")).is_err());
    }

    // Test ensure_symlink_exists()

    #[test]
    fn test_ensure_symlink_exists_initially_nonexitent() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let symlink_path = &tempdir.path().join("link");
        let target_path = Path::new("/");
        ensure_symlink_exists(symlink_path, &target_path).expect("error in ensure_symlink_exists()");
        assert_eq!(target_path, read_link(symlink_path).expect("error reading symlink"));
    }

    #[test]
    fn test_ensure_symlink_exists_initially_correct() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let symlink_path = &tempdir.path().join("link");
        let target_path = Path::new("/");
        symlink(target_path, symlink_path).expect("error creating symlink");
        ensure_symlink_exists(symlink_path, &target_path).expect("error in ensure_symlink_exists()");
        assert_eq!(target_path, read_link(symlink_path).expect("error reading symlink"));
    }

    #[test]
    fn test_ensure_symlink_exists_initially_incorrect() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let symlink_path = &tempdir.path().join("link");
        let target_path = Path::new("/");
        symlink(Path::new("invalid"), symlink_path).expect("error creating symlink");
        ensure_symlink_exists(symlink_path, &target_path).expect("error in ensure_symlink_exists()");
        assert_eq!(target_path, read_link(symlink_path).expect("error reading symlink"));
    }

    // Test file_name_as_number()

    #[test]
    fn test_file_name_as_number() {
        assert_eq!(None, file_name_as_number(Path::new("/abc/jkl/x")));
        assert_eq!(None, file_name_as_number(Path::new("/abc/jkl/x123")));
        assert_eq!(None, file_name_as_number(Path::new("/abc/jkl/123x")));
        assert_eq!(Some(0), file_name_as_number(Path::new("/abc/jkl/0/")));
        assert_eq!(Some(123), file_name_as_number(Path::new("/abc/jkl/123")));
    }

    // Test file_name_is_number()

    #[test]
    fn test_file_name_is_number() {
        assert!(!file_name_is_number(Path::new("/abc/jkl/x")));
        assert!(file_name_is_number(Path::new("/abc/jkl/123")));
    }

    // Test sorted_numeric_entries_in_dir()

    #[test]
    fn test_sorted_numeric_entries_in_dir() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let all_file_names = ["a", "0", "b", "1", "cd", "23", "efg", "456", "jklmno"];
        let numeric_file_names = ["0", "1", "23", "456"];
        let all_file_paths = all_file_names.map(|filename| tempdir.path().join(filename));
        let expected_result: Vec<PathBuf> = numeric_file_names.map(|filename| tempdir.path().join(filename)).into();
        for path in all_file_paths {
            File::create(&path).expect(&format!("failed to create file in tempdir: {}", path.to_str().unwrap()));
        }
        let actual_result = sorted_numeric_entries_in_dir(tempdir.path());
        assert_eq!(expected_result, actual_result);
    }

    // Test is_plus_or_minus_int()

    #[test]
    fn test_is_plus_or_minus_int_false() {
        assert!(!is_plus_or_minus_int(""));
        assert!(!is_plus_or_minus_int("1"));
        assert!(!is_plus_or_minus_int("hi"));
        assert!(!is_plus_or_minus_int("+1."));
        assert!(!is_plus_or_minus_int("-1."));
    }

    #[test]
    fn test_is_plus_or_minus_int_true() {
        assert!(is_plus_or_minus_int("+0"));
        assert!(is_plus_or_minus_int("+1"));
        assert!(is_plus_or_minus_int("-2"));
    }
}
