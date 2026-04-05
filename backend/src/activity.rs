use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub agent: String,
    pub action: String,
    pub path: String,
}

fn log_path(agents_dir: &Path) -> std::path::PathBuf {
    agents_dir.join("activity.jsonl")
}

pub fn append_log(agents_dir: &Path, agent: &str, action: &str, path: &str) {
    let file_path = log_path(agents_dir);
    if let Some(parent) = file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = LogEntry {
        timestamp: now_iso(),
        agent: agent.to_string(),
        action: action.to_string(),
        path: path.to_string(),
    };

    if let Ok(line) = serde_json::to_string(&entry) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&file_path) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

pub fn load_logs(agents_dir: &Path) -> Vec<LogEntry> {
    let file_path = log_path(agents_dir);
    let file = match File::open(&file_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = std::io::BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
            entries.push(entry);
        }
    }

    // Return newest first
    entries.reverse();
    entries
}

fn now_iso() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [
        31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 0u64;
    for (i, &md) in month_days.iter().enumerate() {
        if days < md {
            month = i as u64 + 1;
            break;
        }
        days -= md;
    }
    if month == 0 {
        month = 12;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
