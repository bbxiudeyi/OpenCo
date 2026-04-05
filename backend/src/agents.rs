use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::AgentConfig;

/// Scan agents/ directory and load all agent configs
pub fn load_agents(dir: &Path) -> HashMap<String, AgentConfig> {
    let mut agents = HashMap::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return agents,
    };

    for entry in entries.flatten() {
        let config_path = entry.path().join("config.json");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(cfg) = serde_json::from_str::<AgentConfig>(&content) {
                    agents.insert(cfg.name.clone(), cfg);
                }
            }
        }
    }

    agents
}

/// Validate agent name: no path traversal characters
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Agent name is required".to_string());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Agent name contains invalid characters".to_string());
    }
    Ok(())
}

/// Create a new agent folder with config.json and empty skills/ dir
pub fn create_agent(dir: &Path, config: &AgentConfig) -> Result<(), String> {
    validate_name(&config.name)?;
    let agent_dir = dir.join(&config.name);

    fs::create_dir_all(&agent_dir).map_err(|e| format!("Failed to create agent dir: {}", e))?;
    fs::create_dir_all(agent_dir.join("skills"))
        .map_err(|e| format!("Failed to create skills dir: {}", e))?;

    let config_json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(agent_dir.join("config.json"), config_json)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Delete an agent folder entirely
pub fn delete_agent(dir: &Path, name: &str) -> Result<(), String> {
    validate_name(name)?;
    let agent_dir = dir.join(name);
    if agent_dir.exists() {
        fs::remove_dir_all(&agent_dir).map_err(|e| format!("Failed to delete agent: {}", e))?;
    }
    Ok(())
}

/// Update an existing agent's config.json
pub fn update_agent_config(dir: &Path, config: &AgentConfig) -> Result<(), String> {
    let config_path = dir.join(&config.name).join("config.json");
    let config_json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&config_path, config_json).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}
