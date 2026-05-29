use chrono::{Datelike, Local, NaiveDate, TimeDelta};
use dirs;
use shellexpand::tilde;
use std::{
    env,
    fs::{create_dir_all, exists, read_link},
    path::{Path, PathBuf},
};

use crate::utils::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Command: result of Bydate's command line argument parsing

#[derive(Debug, Eq, PartialEq)]
enum Command {
    Day {
        offset_from_today: i32,
        create_dirs: bool,
    },
    Days {
        offset_from_today: i32,
        only_extant_dirs: bool,
    },
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Bydate: the main program

pub(crate) struct Bydate {
    basedir_path: PathBuf,
    today: NaiveDate,
}

impl Bydate {
    pub(crate) fn new() -> Bydate {
        Bydate {
            basedir_path: Bydate::get_basedir(),
            today: Local::now().naive_local().date(),
        }
    }

    pub(crate) fn main<A: Iterator<Item = String>>(&self, args: A) {
        match Self::parse_args(args) {
            Some(Command::Day {
                offset_from_today,
                create_dirs,
            }) => println!(
                "{}",
                self.get_day(offset_from_today, create_dirs)
                    .to_string_lossy()
            ),
            Some(Command::Days {
                offset_from_today,
                only_extant_dirs,
            }) => {
                for path_buf in self.list_days(offset_from_today, only_extant_dirs) {
                    println!("{}", path_buf.to_string_lossy());
                }
            }
            _ => println!(
                "Usage: bydate {{today [{{+-}}N] [--create-dirs] | yesterday | tomorrow | days {{+-}}N [--extant-dirs] }}"
            ),
        }
    }

    /// Parse arguments:
    ///     ```
    ///     bydate today [+-N] [--no-create-dirs]
    ///     bydate yesterday  =  bydate today -1
    ///     bydate tomorrow  =  bydate today +1
    ///     bydate days +-N [--extant-dirs]
    ///     ```
    fn parse_args<A: Iterator<Item = String>>(mut args: A) -> Option<Command> {
        args.next();
        match args.next().as_deref() {
            Some("today") => {
                let mut offset_from_today: Option<i32> = None;
                let mut create_dirs = true;
                for arg in args {
                    match arg.as_str() {
                        "--no-create-dirs" if create_dirs => create_dirs = false,
                        n if offset_from_today == None && is_plus_or_minus_int(n) => {
                            offset_from_today = Some(n.parse::<i32>().unwrap())
                        }
                        _ => return None,  // Invalid or repeated argument
                    }
                }
                return Some(Command::Day {
                    offset_from_today: offset_from_today.unwrap_or(0),
                    create_dirs,
                });
            }
            Some("yesterday") => Some(Command::Day {
                offset_from_today: -1,
                create_dirs: true,
            }),
            Some("tomorrow") => Some(Command::Day {
                offset_from_today: 1,
                create_dirs: true,
            }),
            Some("days") => {
                let mut offset_from_today: Option<i32> = None;
                let mut only_extant_dirs = false;
                for arg in args {
                    match arg.as_str() {
                        "--extant-dirs" if !only_extant_dirs => only_extant_dirs = true,
                        n if offset_from_today == None && is_plus_or_minus_int(n) => {
                            offset_from_today = Some(n.parse::<i32>().unwrap())
                        }
                        _ => return None,
                    }
                }
                if let Some(offset_from_today) = offset_from_today {
                    return Some(Command::Days {
                        offset_from_today,
                        only_extant_dirs,
                    });
                } else {
                    return None;
                }
            }
            Some(_) | None => None,
        }
    }

    /// Get the parent directory that contains all year directories:
    /// either what the symlink `~/.config/bydate/basedir` points to
    /// or otherwise `~/bydate`.
    fn get_basedir() -> PathBuf {
        if let Some(config_dir_path) = dirs::config_dir() {
            if let Ok(basedir) = read_link(&config_dir_path.join("bydate/basedir")) {
                return basedir;
            }
        }
        return PathBuf::from(tilde("~/bydate").as_ref());
    }

    /// Get the path to the day directory offset by some number of days from self.today
    /// If `create_dirs`, `mkdir -p` the directory.
    /// If `create_dirs` and `offset_from_today == 0`, update the `today` symlink
    fn get_day(&self, offset_from_today: i32, create_dirs: bool) -> PathBuf {
        let day = self.today + TimeDelta::days(offset_from_today as i64);
        let dir_path = self.basedir_path.join(format!(
            "{:04}/{:02}/{:02}",
            day.year(),
            day.month(),
            day.day()
        ));

        if create_dirs {
            let _ = create_dir_all(&dir_path);
            if offset_from_today == 0 {
                // Ignore errors creating symlinks
                let rel_dir_path = dir_path.strip_prefix(&self.basedir_path).ok().unwrap_or(&dir_path);
                let _ = ensure_symlink_exists(&self.basedir_path.join("today"), &rel_dir_path);
            }
        }

        return dir_path;
    }

    /// Take the path to a date directory and parse the date it represents.
    /// Returns `None` if `path` isn't of the form `<basedir>/YYYY/MM/DD`.
    fn parse_date_from_path(&self, path: &Path) -> Option<NaiveDate> {
        let relative_path = path.strip_prefix(&self.basedir_path).ok()?;
        let mut relative_path_components = relative_path.components();
        let year = relative_path_components.next()?.parse_int()?;
        let month = relative_path_components.next()?.parse_int()?;
        let day = relative_path_components.next()?.parse_int()?;
        let nd = NaiveDate::from_ymd_opt(year, month, day);
        return nd;
    }

    /// Finds the earliest date represented by a directory under the basedir.
    fn min_day(&self) -> Option<NaiveDate> {
        for year_path in sorted_numeric_entries_in_dir(&self.basedir_path) {
            for month_path in sorted_numeric_entries_in_dir(&year_path) {
                if let Some(min_year_month_day_dir_path) = numeric_entries_in_dir(&month_path).iter().min() {
                    if let Some(date) = self.parse_date_from_path(&min_year_month_day_dir_path) {
                        return Some(date);
                    }
                }
            }
        }
        None
    }

    /// Finds the latest date represented by a directory under the basedir.
    fn max_day(&self) -> Option<NaiveDate> {
        for year_path in sorted_numeric_entries_in_dir(&self.basedir_path).iter().rev() {
            for month_path in sorted_numeric_entries_in_dir(&year_path).iter().rev() {
                if let Some(min_year_month_day_dir_path) = numeric_entries_in_dir(&month_path).iter().max() {
                    if let Some(date) = self.parse_date_from_path(&min_year_month_day_dir_path) {
                        return Some(date);
                    }
                }
            }
        }
        None
    }

    /// Returns a vector of the days from today through `offset_from_today` days before or after
    /// today (depending on its sign). If `only_extant_dirs` is set, then days that don't have
    /// directories are not included in the returned vector and are not counted.
    fn list_days(&self, offset_from_today: i32, only_extant_dirs: bool) -> Vec<PathBuf> {
        let mut days = Vec::<PathBuf>::new();
        let min_day = self.min_day();
        let max_day = self.max_day();

        if let (Some(min_day), Some(max_day)) = (min_day, max_day) {
            let mut count_remaining = offset_from_today.abs() + 1;
            let mut current_offset = 0;
            let offset_delta = if offset_from_today >= 0 { 1 } else { -1 };

            while count_remaining > 0 {
                let path_buf = self.get_day(current_offset, false);
                current_offset += offset_delta;
                if only_extant_dirs {
                    let day = self.today + TimeDelta::days(current_offset as i64);
                    if day < min_day || day > max_day {
                        break;
                    }
                    if !exists(&path_buf).unwrap_or(false) {
                        continue;
                    }
                }
                days.push(path_buf);
                count_remaining -= 1;
            }
        }

        return days;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    macro_rules! args {
        ( $( $strs: expr ),* ) => {
            ["bydate", $( $strs ),*].into_iter().map(|s| s.to_string())
        }
    }

    fn mkdir_p(dir_path: &Path) {
        create_dir_all(dir_path).expect(&format!("create_dir_all({:?}) failed", dir_path));
    }

    fn create_test_dir_structure(bydate: &Bydate) {
        mkdir_p(&bydate.basedir_path.join("2000"));
        mkdir_p(&bydate.basedir_path.join("2001/01"));
        mkdir_p(&bydate.basedir_path.join("2001/02"));
        mkdir_p(&bydate.basedir_path.join("2002/03/04"));
        mkdir_p(&bydate.basedir_path.join("2002/05/06"));
        mkdir_p(&bydate.basedir_path.join("2003/07"));
        mkdir_p(&bydate.basedir_path.join("2003/08"));
        mkdir_p(&bydate.basedir_path.join("2004"));
    }

    // Test Bydate::parse_args()

    #[test]
    fn test_bydate_parse_args() {
        assert_eq!(None, Bydate::parse_args([].into_iter()));
        // bydate today ...
        assert_eq!(Some(Command::Day { offset_from_today: 0, create_dirs: true   }), Bydate::parse_args(args!("today")));
        assert_eq!(Some(Command::Day { offset_from_today: 3, create_dirs: true   }), Bydate::parse_args(args!("today", "+3")));
        assert_eq!(Some(Command::Day { offset_from_today: -3, create_dirs: true  }), Bydate::parse_args(args!("today", "-3")));
        assert_eq!(None, Bydate::parse_args(args!("today", "3")));
        assert_eq!(Some(Command::Day { offset_from_today: 0, create_dirs: false  }), Bydate::parse_args(args!("today", "--no-create-dirs")));
        assert_eq!(None, Bydate::parse_args(args!("today", "--invalid-arg")));
        assert_eq!(Some(Command::Day { offset_from_today: 3, create_dirs: false  }), Bydate::parse_args(args!("today", "+3", "--no-create-dirs")));
        assert_eq!(Some(Command::Day { offset_from_today: -3, create_dirs: false }), Bydate::parse_args(args!("today", "--no-create-dirs", "-3")));
        // bydate yesterday ...
        // TODO
        // bydate tomorrow ...
        // TODO
        // bydate days ...
        // TODO
    }

    // Test Bydate::min_day()

    #[test]
    fn test_min_day() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let bydate = Bydate { basedir_path: tempdir.path().to_path_buf(), today: NaiveDate::from_ymd_opt(2010, 1, 1).unwrap() };
        create_test_dir_structure(&bydate);
        assert_eq!(Some(NaiveDate::from_ymd_opt(2002, 3, 4).unwrap()), bydate.min_day());
    }

    // Test Bydate::max_day()

    #[test]
    fn test_max_day() {
        let tempdir = tempdir().expect("failed to create temporary directory with tempdir()");
        let bydate = Bydate { basedir_path: tempdir.path().to_path_buf(), today: NaiveDate::from_ymd_opt(2010, 1, 1).unwrap() };
        create_test_dir_structure(&bydate);
        assert_eq!(Some(NaiveDate::from_ymd_opt(2002, 5, 6).unwrap()), bydate.max_day());
    }
}
