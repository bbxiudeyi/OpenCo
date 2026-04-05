use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A single persisted message entry
#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

fn history_path(agents_dir: &Path, agent_name: &str) -> std::path::PathBuf {
    agents_dir.join(agent_name).join("history.jsonl")
}

/// Append a message to an agent's history file
pub fn append_message(agents_dir: &Path, agent_name: &str, role: &str, content: &str) {
    let path = history_path(agents_dir, agent_name);

    // Ensure agent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = HistoryEntry {
        role: role.to_string(),
        content: content.to_string(),
        timestamp: Some(now_iso()),
    };

    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open history file: {}", e);
            return;
        }
    };

    if let Ok(line) = serde_json::to_string(&entry) {
        let _ = writeln!(file, "{}", line);
    }
}

/// Append user + assistant exchange to history
pub fn append_exchange(
    agents_dir: &Path,
    agent_name: &str,
    user_msg: &str,
    assistant_msg: &str,
) {
    append_message(agents_dir, agent_name, "user", user_msg);
    append_message(agents_dir, agent_name, "assistant", assistant_msg);
}

/// Load all history entries for an agent
pub fn load_history(agents_dir: &Path, agent_name: &str) -> Vec<HistoryEntry> {
    let path = history_path(agents_dir, agent_name);
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = std::io::BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
                entries.push(entry);
            }
        }
    }

    entries
}

/// Clear all history for an agent
pub fn clear_history(agents_dir: &Path, agent_name: &str) -> Result<(), String> {
    let path = history_path(agents_dir, agent_name);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to clear history: {}", e))?;
    }
    Ok(())
}

fn now_iso() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple ISO-like timestamp
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    // Approximate year calculation (from 1970)
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
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_timestamp() {
        let ts = now_iso();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }
}
