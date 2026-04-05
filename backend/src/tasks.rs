use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String, // "pending", "in_progress", "done"
    pub agents: Vec<String>,
    pub progress: u32, // 0-100
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TaskBoard {
    pub tasks: Vec<Task>,
}

fn gen_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let rand_part: u16 = (ts & 0xFFFF) as u16 ^ ((ts >> 16) & 0xFFFF) as u16;
    format!("task_{}_{:04x}", ts, rand_part)
}

pub fn load_tasks(path: &Path) -> TaskBoard {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_tasks(path: &Path, board: &TaskBoard) {
    if let Ok(s) = serde_json::to_string_pretty(board) {
        let _ = fs::write(path, s);
    }
}

pub fn add_task(board: &mut TaskBoard, title: String, description: String) -> Task {
    let task = Task {
        id: gen_id(),
        title,
        description,
        status: "in_progress".to_string(),
        agents: Vec::new(),
        progress: 0,
    };
    board.tasks.push(task.clone());
    task
}

pub fn update_task(board: &mut TaskBoard, id: &str, title: Option<String>, description: Option<String>, status: Option<String>, agents: Option<Vec<String>>, progress: Option<u32>) -> bool {
    if let Some(task) = board.tasks.iter_mut().find(|t| t.id == id) {
        if let Some(v) = title { task.title = v; }
        if let Some(v) = description { task.description = v; }
        if let Some(v) = status { task.status = v; }
        if let Some(v) = agents { task.agents = v; }
        if let Some(v) = progress { task.progress = v.min(100); }
        return true;
    }
    false
}

pub fn delete_task(board: &mut TaskBoard, id: &str) -> bool {
    let before = board.tasks.len();
    board.tasks.retain(|t| t.id != id);
    board.tasks.len() < before
}
