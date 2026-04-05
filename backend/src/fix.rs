use std::fs;
use std::path::Path;

use crate::config::{AgentConfig, Config, OrgChart};
use crate::tasks::TaskBoard;

/// Run the fix command: repair data files and check installation integrity.
pub fn run_fix(data_dir: &Path) {
    println!("OpenCo fix — checking data directory: {}", data_dir.display());

    let mut fixed = 0;
    let mut errors = 0;

    // 1. Ensure directory structure exists
    ensure_dir(data_dir, &mut fixed);
    let agents_dir = data_dir.join("agents");
    let workspace_dir = data_dir.join("workspace");
    ensure_dir(&agents_dir, &mut fixed);
    ensure_dir(&workspace_dir, &mut fixed);

    // 2. Fix config.json
    let config_path = data_dir.join("config.json");
    match fix_json_file::<Config>(&config_path, &Config::default(), "config.json") {
        FixResult::Fixed => {
            fixed += 1;
            println!("  [FIXED] config.json — reset to default");
        }
        FixResult::Ok => println!("  [OK] config.json"),
        FixResult::Error(e) => {
            errors += 1;
            println!("  [ERROR] config.json — {}", e);
        }
    }

    // 3. Fix org.json
    let org_path = agents_dir.join("org.json");
    match fix_json_file::<OrgChart>(&org_path, &OrgChart::default(), "org.json") {
        FixResult::Fixed => {
            fixed += 1;
            println!("  [FIXED] org.json — reset to default");
        }
        FixResult::Ok => println!("  [OK] org.json"),
        FixResult::Error(e) => {
            errors += 1;
            println!("  [ERROR] org.json — {}", e);
        }
    }

    // 4. Fix tasks.json
    let tasks_path = agents_dir.join("tasks.json");
    match fix_json_file::<TaskBoard>(&tasks_path, &TaskBoard::default(), "tasks.json") {
        FixResult::Fixed => {
            fixed += 1;
            println!("  [FIXED] tasks.json — reset to default");
        }
        FixResult::Ok => println!("  [OK] tasks.json"),
        FixResult::Error(e) => {
            errors += 1;
            println!("  [ERROR] tasks.json — {}", e);
        }
    }

    // 5. Fix workspace/access.json
    let access_path = workspace_dir.join("access.json");
    match fix_json_file::<serde_json::Value>(&access_path, &serde_json::json!({}), "workspace/access.json") {
        FixResult::Fixed => {
            fixed += 1;
            println!("  [FIXED] workspace/access.json — reset to default");
        }
        FixResult::Ok => println!("  [OK] workspace/access.json"),
        FixResult::Error(e) => {
            errors += 1;
            println!("  [ERROR] workspace/access.json — {}", e);
        }
    }

    // 6. Fix agent configs
    if let Ok(entries) = fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let cfg_path = entry.path().join("config.json");
                if cfg_path.exists() {
                    let default_cfg = AgentConfig {
                        name: name.clone(),
                        ..Default::default()
                    };
                    match fix_agent_config(&cfg_path, &default_cfg, &name) {
                        FixResult::Fixed => {
                            fixed += 1;
                            println!("  [FIXED] agents/{}/config.json — repaired", name);
                        }
                        FixResult::Ok => println!("  [OK] agents/{}/config.json", name),
                        FixResult::Error(e) => {
                            errors += 1;
                            println!("  [ERROR] agents/{}/config.json — {}", name, e);
                        }
                    }
                }
            }
        }
    }

    // 7. Check installation integrity — frontend files
    println!("\n--- Installation Integrity ---");
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()));

    if let Some(ref exe_dir) = exe_dir {
        check_frontend_files(exe_dir, &mut errors);
    } else {
        // Dev mode: check relative to project root
        println!("  [INFO] Running in development mode, skipping frontend check");
    }

    // Summary
    println!("\n--- Summary ---");
    if fixed == 0 && errors == 0 {
        println!("All checks passed. No issues found.");
    } else {
        if fixed > 0 {
            println!("  {} file(s) fixed.", fixed);
        }
        if errors > 0 {
            println!("  {} error(s) could not be auto-fixed.", errors);
        }
    }
}

enum FixResult {
    Ok,
    Fixed,
    Error(String),
}

fn ensure_dir(dir: &Path, fixed: &mut usize) {
    if !dir.exists() {
        match fs::create_dir_all(dir) {
            Ok(()) => {
                *fixed += 1;
                println!("  [CREATED] {}", dir.display());
            }
            Err(e) => println!("  [ERROR] Failed to create {}: {}", dir.display(), e),
        }
    }
}

/// Try to parse a JSON file. If it fails, overwrite with default value.
fn fix_json_file<T: serde::Serialize + serde::de::DeserializeOwned>(
    path: &Path,
    default: &T,
    _label: &str,
) -> FixResult {
    if !path.exists() {
        // File doesn't exist — create with default
        match write_json(path, default) {
            Ok(()) => FixResult::Fixed,
            Err(e) => FixResult::Error(format!("cannot create: {}", e)),
        }
    } else {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<T>(&content) {
                Ok(_) => FixResult::Ok,
                Err(_) => {
                    // Corrupted — backup and rewrite
                    let _ = fs::rename(path, path.with_extension("json.bak"));
                    match write_json(path, default) {
                        Ok(()) => FixResult::Fixed,
                        Err(e) => FixResult::Error(format!("cannot rewrite: {}", e)),
                    }
                }
            },
            Err(e) => FixResult::Error(format!("cannot read: {}", e)),
        }
    }
}

/// Agent config fix: try partial parse, fill missing fields.
fn fix_agent_config(path: &Path, default: &AgentConfig, name: &str) -> FixResult {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return FixResult::Error(format!("cannot read: {}", e)),
    };

    // Try full parse first
    if serde_json::from_str::<AgentConfig>(&content).is_ok() {
        return FixResult::Ok;
    }

    // Try partial: parse as Value, fill missing fields, write back
    let mut val = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(v) => v,
        Err(_) => {
            // Not even valid JSON — reset to default
            let _ = fs::rename(path, path.with_extension("json.bak"));
            return match write_json(path, default) {
                Ok(()) => FixResult::Fixed,
                Err(e) => FixResult::Error(format!("cannot rewrite: {}", e)),
            };
        }
    };

    // Ensure required fields exist
    let default_val = serde_json::to_value(default).unwrap_or_default();
    if let (serde_json::Value::Object(map), serde_json::Value::Object(def_map)) =
        (&mut val, &default_val)
    {
        let mut needed_fix = false;
        for key in ["name", "model", "api_url", "api_key", "system_prompt", "tools"] {
            if !map.contains_key(key) {
                if let Some(v) = def_map.get(key) {
                    map.insert(key.to_string(), v.clone());
                    needed_fix = true;
                }
            }
        }
        // Ensure name is correct
        if let Some(n) = map.get_mut("name") {
            *n = serde_json::Value::String(name.to_string());
        }

        if needed_fix {
            let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
            match fs::write(path, pretty) {
                Ok(()) => FixResult::Fixed,
                Err(e) => FixResult::Error(format!("cannot write: {}", e)),
            }
        } else {
            FixResult::Ok
        }
    } else {
        FixResult::Ok
    }
}

fn write_json<T: serde::Serialize>(path: &Path, data: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)
}

fn check_frontend_files(base: &Path, errors: &mut usize) {
    let checks = [
        ("frontend/index.html", false),
        ("frontend/style.css", false),
        ("frontend/js/app.js", false),
    ];

    for (rel_path, _is_dir) in &checks {
        let full = base.join(rel_path);
        if full.exists() {
            println!("  [OK] {}", rel_path);
        } else {
            *errors += 1;
            println!("  [MISSING] {} — installation may be incomplete", rel_path);
        }
    }
}
