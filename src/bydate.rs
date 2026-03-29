use chrono::{Datelike, Local, NaiveDate, TimeDelta};
use dirs;
use shellexpand::tilde;
use std::{env, fs::{create_dir_all, exists, read_link, remove_file}, path::{Path, PathBuf}};

use crate::utils::{dir_contents, ensure_symlink, file_name_is_number, is_plus_or_minus_int, ParseInt};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Command: result of Bydate's command line argument parsing

enum Command {
    Day { offset_from_today: i32, create_dirs: bool },
    Days { offset_from_today: i32, only_extant_dirs: bool },
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Bydate: the main program

pub(crate) struct Bydate {
    basedir_path: PathBuf,
    today: NaiveDate,
}

impl Bydate {
    pub(crate) fn new() -> Bydate {
        Bydate { basedir_path: Bydate::get_basedir(), today: Local::now().naive_local().date() }
    }

    pub(crate) fn main(&self) {
        match Self::parse_args() {
            Some(Command::Day { offset_from_today, create_dirs: create_dir }) =>
                println!("{}", self.get_day(offset_from_today, create_dir).to_string_lossy()),
            Some(Command::Days { offset_from_today, only_extant_dirs }) =>
                for path_buf in self.list_days(offset_from_today, only_extant_dirs) {
                    println!("{}", path_buf.to_string_lossy());
                }
            _ => println!("Usage: bydate {{today [{{+-}}N] [--create-dirs] | yesterday | tomorrow | days {{+-}}N [--extant-dirs] }}")
        }
    }

    fn parse_args() -> Option<Command> {
        // bydate today [+-N] [--no-create-dirs]
        // bydate yesterday = bydate today -1
        // bydate tomorrow = bydate today +1
        // bydate days +-N [--extant-dirs]
        let mut args = env::args();
        args.next();
        match args.next().as_deref() {
            Some("today") => {
                    let mut offset_from_today: Option<i32> = None;
                    let mut create_dirs = true;
                    for arg in args {
                        match arg.as_str() {
                            "--create-dirs" if create_dirs => create_dirs = false,
                            n if offset_from_today == None && is_plus_or_minus_int(n) => offset_from_today = Some(n.parse::<i32>().unwrap()),
                            _ => return None,
                        }
                    }
                    return Some(Command::Day { offset_from_today: offset_from_today.unwrap_or(0), create_dirs });
            },
            Some("yesterday") => Some(Command::Day { offset_from_today: -1, create_dirs: true }),
            Some("tomorrow") => Some(Command::Day { offset_from_today: 1, create_dirs: true }),
            Some("days") => {
                    let mut offset_from_today: Option<i32> = None;
                    let mut only_extant_dirs = false;
                    for arg in args {
                        match arg.as_str() {
                            "--extant-dirs" if !only_extant_dirs => only_extant_dirs = true,
                            n if offset_from_today == None && is_plus_or_minus_int(n) => offset_from_today = Some(n.parse::<i32>().unwrap()),
                            _ => return None,
                        }
                    }
                    if let Some(offset_from_today) = offset_from_today {
                        return Some(Command::Days { offset_from_today, only_extant_dirs });
                    } else {
                        return None;
                    }
            },
            Some(_) | None => None,
        }
    }

    fn get_basedir() -> PathBuf { 
        if let Some(config_dir_path) = dirs::config_dir() {
            let basedir_link_path = config_dir_path.join("bydate/basedir");
            if let Ok(basedir) = read_link(&basedir_link_path) {
                return basedir;
            }
        }
        return PathBuf::from(tilde("~/bydate").as_ref());
    }

    fn get_day(&self, offset_from_today: i32, create_dirs: bool) -> PathBuf {
        let day = self.today + TimeDelta::days(offset_from_today as i64);
        let dir_path = self.basedir_path.join(format!("{:04}/{:02}/{:02}", day.year(), day.month(), day.day()));

        if create_dirs {
            let _ = create_dir_all(&dir_path);
            if offset_from_today == 0 {
                let today_symlink_path = self.basedir_path.join("today");
                if exists(&dir_path).unwrap_or(false) {
                    ensure_symlink(&today_symlink_path, &dir_path);
                } else {
                    let _ = remove_file(today_symlink_path);
                }
            }
        }

        return dir_path;
    }

    fn parse_date_from_path(&self, path: &Path) -> Option<NaiveDate> {
        let relative_path = path.strip_prefix(&self.basedir_path).ok()?;
        let mut relative_path_components = relative_path.components();
        let year = relative_path_components.next()?.as_os_str().parse_int()?;
        let month = relative_path_components.next()?.as_os_str().parse_int()?;
        let day = relative_path_components.next()?.as_os_str().parse_int()?;
        let nd = NaiveDate::from_ymd_opt(year, month, day);
        return nd;
    }

    fn min_day(&self) -> Option<NaiveDate> {
        let min_num_in_dir =
            |p| dir_contents(p).ok()?.filter(|p| file_name_is_number(p)).min();
        let min_year_dir_path = min_num_in_dir(&self.basedir_path)?;
        let min_year_month_dir_path = min_num_in_dir(&min_year_dir_path)?;
        let min_year_month_day_dir_path = min_num_in_dir(&min_year_month_dir_path)?;
        return self.parse_date_from_path(&min_year_month_day_dir_path);
    }

    fn max_day(&self) -> Option<NaiveDate> {
        let max_num_in_dir =
            |p| dir_contents(p).ok()?.filter(|p| file_name_is_number(p)).min();
        let max_year_dir_path = max_num_in_dir(&self.basedir_path)?;
        let max_year_month_dir_path = max_num_in_dir(&max_year_dir_path)?;
        let max_year_month_day_dir_path = max_num_in_dir(&max_year_month_dir_path)?;
        return self.parse_date_from_path(&max_year_month_day_dir_path);
    }

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
                        break
                    }
                    if !exists(&path_buf).unwrap_or(false) {
                        continue
                    }
                }
                days.push(path_buf);
                count_remaining -= 1;
            }
        }

        return days;
    }
}
