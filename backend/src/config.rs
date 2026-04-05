use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::tasks::TaskBoard;

/// A named model configuration entry
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ModelEntry {
    pub name: String,
    pub provider: String,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
}

/// Global config (served by /api/config)
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub models: Vec<ModelEntry>,
    #[serde(default)]
    pub default_model: Option<String>,
}

impl Config {
    /// Resolve a model entry by name, or fall back to the default model.
    pub fn resolve_model(&self, model_name: Option<&str>) -> Option<&ModelEntry> {
        let name = model_name.or(self.default_model.as_deref());
        name.and_then(|n| self.models.iter().find(|m| m.name == n))
    }
}

/// Per-agent configuration stored in agents/<name>/config.json
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub api_url: String,
    pub api_key: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
    #[serde(default)]
    pub model_name: Option<String>,
}

/// Workspace access control
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WorkspaceAccess {
    pub projects: HashMap<String, ProjectAccess>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectAccess {
    pub agents: Vec<String>,
}

/// Global application state
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_path: std::path::PathBuf,
    pub agents: Arc<Mutex<HashMap<String, AgentConfig>>>,
    pub workspace_access: Arc<Mutex<WorkspaceAccess>>,
    pub agents_dir: std::path::PathBuf,
    pub workspace_dir: std::path::PathBuf,
    pub org: Arc<Mutex<OrgChart>>,
    pub org_path: std::path::PathBuf,
    pub tasks: Arc<Mutex<TaskBoard>>,
    pub tasks_path: std::path::PathBuf,
}

/// Organization chart - company structure
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct OrgChart {
    pub positions: Vec<Position>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Position {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub agents: Vec<String>,
    pub system_prompt: String,
}
