pub mod delegate;
pub mod exec;
pub mod read_file;
pub mod web_search;
pub mod write_file;

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Cached full tool definitions — built once, reused forever
static ALL_TOOLS: OnceLock<Vec<Value>> = OnceLock::new();

fn all_tool_definitions() -> &'static [Value] {
    ALL_TOOLS.get_or_init(|| {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "web_search",
                    "description": "Search the web for information. Returns search result snippets.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query" }
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file from disk.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "File path to read" }
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write content to a file on disk.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "File path to write" },
                            "content": { "type": "string", "description": "Content to write" }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "exec",
                    "description": "Execute a shell command and return its output. Use for running programs, scripts, or system commands.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "Shell command to execute" }
                        },
                        "required": ["command"]
                    }
                }
            }),
        ]
    })
}

/// Get tool definitions JSON filtered by allowed tool names.
/// If `allowed` is empty, returns all tools (backward compatible).
pub fn get_tools_json(allowed: &[String]) -> Value {
    let all = all_tool_definitions();

    if allowed.is_empty() {
        return Value::Array(all.to_vec());
    }

    let tools: Vec<Value> = all
        .iter()
        .filter(|t| {
            let name = t["function"]["name"].as_str().unwrap_or("");
            allowed.iter().any(|a| a == name)
        })
        .cloned()
        .collect();

    Value::Array(tools)
}

/// Pre-resolve allowed dirs to canonical paths (call once per request)
pub fn resolve_allowed_dirs(dirs: &[PathBuf]) -> Vec<PathBuf> {
    dirs.iter()
        .map(|d| std::fs::canonicalize(d).unwrap_or_else(|_| d.clone()))
        .collect()
}

/// Check if a file path falls under any of the pre-resolved allowed directories.
fn is_path_allowed(path: &str, resolved_dirs: &[PathBuf]) -> bool {
    if resolved_dirs.is_empty() {
        return true;
    }

    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => {
            let p = PathBuf::from(path);
            match p.parent() {
                Some(parent) if parent.as_os_str().is_empty() => return false,
                Some(parent) => match std::fs::canonicalize(parent) {
                    Ok(cp) => cp,
                    Err(_) => return false,
                },
                None => return true,
            }
        }
    };

    resolved_dirs.iter().any(|dir| canonical.starts_with(dir))
}

/// Execute a tool with optional sandbox restrictions.
/// `resolved_dirs` should be pre-canonicalized via `resolve_allowed_dirs`.
pub fn execute_tool(name: &str, arguments: &str, resolved_dirs: &[PathBuf]) -> String {
    let args: Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(_) => return format!("Invalid arguments: {}", arguments),
    };

    match name {
        "web_search" => {
            let query = args["query"].as_str().unwrap_or("");
            web_search::tool_web_search(query)
        }
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            if !is_path_allowed(path, resolved_dirs) {
                return "Access denied: path is outside allowed workspace".to_string();
            }
            read_file::tool_read_file(path)
        }
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("");
            if !is_path_allowed(path, resolved_dirs) {
                return "Access denied: path is outside allowed workspace".to_string();
            }
            let content = args["content"].as_str().unwrap_or("");
            write_file::tool_write_file(path, content)
        }
        "exec" => {
            let command = args["command"].as_str().unwrap_or("");
            if !resolved_dirs.is_empty() {
                exec::tool_exec_in_dir(command, &resolved_dirs[0])
            } else {
                exec::tool_exec(command)
            }
        }
        _ => format!("Unknown tool: {}", name),
    }
}
