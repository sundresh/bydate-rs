use chrono::{Datelike, Days, Local};
use dirs;
use shellexpand::tilde;
use std::{env, fs::{create_dir_all, exists, read_link, remove_file}, os::unix::fs::symlink, path::{Path, PathBuf}};

enum Command {
    Day { offset_from_today: i32, create_dirs: bool },
    Days { offset_from_today: i32, only_extant_dirs: bool },
}

fn is_plus_or_minus_int(s: &str) -> bool {
    (s.starts_with("+") || s.starts_with("-")) && s.parse::<i32>().is_ok()
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
                Some(Command::Day { offset_from_today: offset_from_today.unwrap_or(0), create_dirs })
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
                    Some(Command::Days { offset_from_today, only_extant_dirs })
                } else {
                    None
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

fn ensure_symlink<P: AsRef<Path>, Q: AsRef<Path>>(symlink_path: P, target_path: Q) {
    // Updates are currently non-atomic
    if let Ok(current_target_path) = read_link(&symlink_path) {
        if current_target_path.as_path() == target_path.as_ref() {
            return
        } else {
            let _ = remove_file(&symlink_path);
        }
    }
    let _ = symlink(target_path, symlink_path);
}

fn get_day(offset_from_today: i32, create_dirs: bool) -> PathBuf {
    let now = Local::now();
    let day = if offset_from_today >= 0 {
        now.checked_add_days(Days::new(offset_from_today as u64)).unwrap()
    } else {
        now.checked_sub_days(Days::new(-offset_from_today as u64)).unwrap()
    };
    let basedir_path = get_basedir();
    let dir_path = basedir_path.join(format!("{:04}/{:02}/{:02}", day.year(), day.month(), day.day()));
    if create_dirs {
        let _ = create_dir_all(&dir_path);
    }
    if offset_from_today == 0 {
        let today_symlink_path = basedir_path.join("today");
        if exists(&dir_path).ok().unwrap_or(false) {
            ensure_symlink(today_symlink_path, &dir_path);
        } else {
            let _ = remove_file(today_symlink_path);
        }
    }
    dir_path
}

fn main() {
    match parse_args() {
        Some(Command::Day { offset_from_today, create_dirs: create_dir }) =>
            println!("{}", get_day(offset_from_today, create_dir).to_string_lossy()   ),
        _ => println!("Usage: bydate {{today [+-N] [--create-dirs] | yesterday | tomorrow}}")
    }
}
