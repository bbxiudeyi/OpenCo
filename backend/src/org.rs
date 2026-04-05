use std::fs;
use std::path::Path;

use crate::config::{AgentConfig, OrgChart, Position};

/// Load org.json
pub fn load_org(path: &Path) -> OrgChart {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => OrgChart::default(),
    }
}

/// Save org.json
pub fn save_org(path: &Path, org: &OrgChart) -> Result<(), String> {
    let dir = path.parent().unwrap_or(path);
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let json =
        serde_json::to_string_pretty(org).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write org.json: {}", e))?;
    Ok(())
}

/// Generate a unique position ID (timestamp + random suffix)
fn gen_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // Simple random suffix without external crate
    let rand_part = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()) & 0xFFFF;
    format!("pos_{}_{:04x}", ts, rand_part)
}

/// Add a new position
pub fn add_position(org: &mut OrgChart, title: String, parent_id: Option<String>) -> Position {
    let pos = Position {
        id: gen_id(),
        title,
        parent_id,
        agents: Vec::new(),
        system_prompt: String::new(),
    };
    org.positions.push(pos.clone());
    pos
}

/// Remove a position. Children are re-parented to the removed position's parent.
pub fn remove_position(org: &mut OrgChart, id: &str) -> Result<(), String> {
    let parent_id: Option<String> = org
        .positions
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.parent_id.clone())
        .ok_or("Position not found")?;

    // Re-parent children
    for pos in &mut org.positions {
        if pos.parent_id.as_deref() == Some(id) {
            pos.parent_id = parent_id.clone();
        }
    }

    // Remove the position
    org.positions.retain(|p| p.id != id);
    Ok(())
}

/// Update a position's fields
pub fn update_position(
    org: &mut OrgChart,
    id: &str,
    title: Option<String>,
    parent_id: Option<Option<String>>,
    agents: Option<Vec<String>>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    let pos = org
        .positions
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Position not found")?;

    if let Some(t) = title {
        pos.title = t;
    }
    if let Some(p) = parent_id {
        pos.parent_id = p;
    }
    if let Some(a) = agents {
        pos.agents = a;
    }
    if let Some(s) = system_prompt {
        pos.system_prompt = s;
    }
    Ok(())
}

/// Get direct subordinates of a position
pub fn get_subordinates(org: &OrgChart, position_id: &str) -> Vec<Position> {
    org.positions
        .iter()
        .filter(|p| p.parent_id.as_deref() == Some(position_id))
        .cloned()
        .collect()
}

/// Find a position by ID
pub fn find_position(org: &OrgChart, id: &str) -> Option<Position> {
    org.positions.iter().find(|p| p.id == id).cloned()
}

/// Find the position an agent is assigned to
pub fn find_agent_position(org: &OrgChart, agent_name: &str) -> Option<Position> {
    org.positions
        .iter()
        .find(|p| p.agents.contains(&agent_name.to_string()))
        .cloned()
}

/// Build a HashMap from agent name → Position for O(1) lookup
pub fn build_agent_position_index(org: &OrgChart) -> std::collections::HashMap<String, Position> {
    let mut map = std::collections::HashMap::new();
    for pos in &org.positions {
        for agent_name in &pos.agents {
            map.entry(agent_name.clone()).or_insert_with(|| pos.clone());
        }
    }
    map
}

/// Build the combined system prompt for an agent in a position
pub fn build_system_prompt(
    org: &OrgChart,
    position: &Position,
    agent: &AgentConfig,
) -> String {
    let mut parts = Vec::new();

    // Position context
    parts.push(format!("【职位】你是公司的{}。", position.title));

    // Parent info
    if let Some(ref parent_id) = position.parent_id {
        if let Some(parent) = find_position(org, parent_id) {
            let parent_names: Vec<&str> = parent.agents.iter().map(|s| s.as_str()).collect();
            if !parent_names.is_empty() {
                parts.push(format!("你的上级是：{}。", parent_names.join("、")));
            } else {
                parts.push(format!("你的上级是：{}。", parent.title));
            }
        }
    } else {
        parts.push("你是最高管理者，直接向用户（公司所有者）汇报。".to_string());
    }

    // Subordinates info
    let subs = get_subordinates(org, &position.id);
    if !subs.is_empty() {
        let sub_names: Vec<String> = subs.iter().flat_map(|s| s.agents.clone()).collect();
        if !sub_names.is_empty() {
            parts.push(format!("你的下属有：{}。你可以使用delegate工具向他们分配任务。", sub_names.join("、")));
        } else {
            let sub_titles: Vec<&str> = subs.iter().map(|s| s.title.as_str()).collect();
            parts.push(format!("你的下属有：{}。你可以使用delegate工具向他们分配任务。", sub_titles.join("、")));
        }
    }

    // Position-specific prompt
    if !position.system_prompt.is_empty() {
        parts.push(format!("【职位要求】{}", position.system_prompt));
    }

    // Agent personal prompt
    if !agent.system_prompt.is_empty() {
        parts.push(format!("【个人设定】{}", agent.system_prompt));
    }

    parts.join("\n\n")
}
