use std::fs;
use std::path::Path;

use crate::config::{ProjectAccess, WorkspaceAccess};

/// Load workspace/access.json
pub fn load_access(path: &Path) -> WorkspaceAccess {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => WorkspaceAccess::default(),
    }
}

/// Save workspace/access.json
pub fn save_access(path: &Path, access: &WorkspaceAccess) -> Result<(), String> {
    let dir = path.parent().unwrap_or(path);
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create workspace dir: {}", e))?;

    let json = serde_json::to_string_pretty(access)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write access.json: {}", e))?;
    Ok(())
}

/// Grant an agent access to a project
pub fn grant_access(access: &mut WorkspaceAccess, project: &str, agent: &str) {
    let entry = access
        .projects
        .entry(project.to_string())
        .or_insert_with(ProjectAccess::default);

    if !entry.agents.contains(&agent.to_string()) {
        entry.agents.push(agent.to_string());
    }
}

/// Revoke an agent's access to a project
pub fn revoke_access(access: &mut WorkspaceAccess, project: &str, agent: &str) {
    if let Some(pa) = access.projects.get_mut(project) {
        pa.agents.retain(|a| a != agent);
        if pa.agents.is_empty() {
            access.projects.remove(project);
        }
    }
}

/// Get all projects an agent can access
pub fn get_agent_projects(access: &WorkspaceAccess, agent: &str) -> Vec<String> {
    access
        .projects
        .iter()
        .filter(|(_, pa)| pa.agents.contains(&agent.to_string()))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Check if an agent has access to a project
pub fn check_access(access: &WorkspaceAccess, project: &str, agent: &str) -> bool {
    access
        .projects
        .get(project)
        .map_or(false, |pa| pa.agents.contains(&agent.to_string()))
}

/// List all project directory names under workspace/
pub fn list_projects(workspace_dir: &Path) -> Vec<String> {
    let entries = match fs::read_dir(workspace_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|name| name != "." && name != "..")
        .collect()
}
