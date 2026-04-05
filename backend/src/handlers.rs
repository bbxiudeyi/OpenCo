use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use axum::extract::{Path as AxumPath, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::activity;
use crate::agents;
use crate::config::{AgentConfig, AppState, Config, ModelEntry, OrgChart, WorkspaceAccess};
use crate::history;
use crate::org;
use crate::tasks;
use crate::tools;
use crate::workspace;

// ===== SSE Helpers =====

type SseSender = tokio::sync::mpsc::Sender<Result<Event, Infallible>>;

/// Context needed for the delegate tool inside the SSE chat loop
struct DelegateContext {
    org: OrgChart,
    agents: std::collections::HashMap<String, AgentConfig>,
    models: Vec<ModelEntry>,
    global_config: Config,
}

fn send_sse(tx: &SseSender, event_type: &str, data: &str) {
    let event = Event::default()
        .event(event_type)
        .data(data.to_string());
    let _ = tx.blocking_send(Ok(event));
}

fn chat_with_tools_sse(
    api_url: &str,
    api_key: &str,
    model: &str,
    tools_json: Value,
    allowed_dirs: Vec<PathBuf>,
    mut messages: Vec<Value>,
    system_prompt: Option<String>,
    tx: SseSender,
    reply_out: Arc<std::sync::Mutex<Option<String>>>,
    delegate_ctx: Option<DelegateContext>,
    agent_name: Option<&str>,
    agents_dir: Option<&Path>,
) {
    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));

    // Inject system prompt if provided
    if let Some(sp) = system_prompt {
        if !sp.is_empty() {
            messages.insert(
                0,
                json!({
                    "role": "system",
                    "content": sp
                }),
            );
        }
    }

    let max_rounds = 10;

    for _round in 0..max_rounds {
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools_json,
            "tool_choice": "auto"
        });

        println!("LLM request (round {})", _round + 1);

        let result = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .set("Content-Type", "application/json")
            .send_string(&body.to_string());

        let resp_body: String = match result {
            Ok(resp) => resp.into_string().unwrap_or_default(),
            Err(ureq::Error::Status(_code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                send_sse(&tx, "error", &json!({"error": err_body}).to_string());
                return;
            }
            Err(e) => {
                send_sse(&tx, "error", &json!({"error": e.to_string()}).to_string());
                return;
            }
        };

        let resp_json: Value = match serde_json::from_str(&resp_body) {
            Ok(v) => v,
            Err(e) => {
                send_sse(&tx, "error", &json!({"error": e.to_string()}).to_string());
                return;
            }
        };

        let choice = &resp_json["choices"][0];
        let msg = &choice["message"];

        // Check if there are tool_calls
        if let Some(tc) = msg.get("tool_calls") {
            if tc.as_array().map_or(false, |a| !a.is_empty()) {
                messages.push(msg.clone());

                if let Some(calls) = tc.as_array() {
                    for call in calls {
                        let tool_name =
                            call["function"]["name"].as_str().unwrap_or("unknown");
                        let tool_args =
                            call["function"]["arguments"].as_str().unwrap_or("{}");
                        let tool_call_id = call["id"].as_str().unwrap_or("");

                        println!("Tool call: {} ({})", tool_name, tool_args);

                        // Send "running" event immediately
                        send_sse(
                            &tx,
                            "step",
                            &json!({
                                "name": tool_name,
                                "input": tool_args,
                                "status": "running"
                            })
                            .to_string(),
                        );

                        // Execute the tool with sandbox (or delegate)
                        let output = if tool_name == "delegate" {
                            if let Some(ref ctx) = delegate_ctx {
                                let args: Value = serde_json::from_str(tool_args).unwrap_or_default();
                                let target = args["agent_name"].as_str().unwrap_or("");
                                let task_desc = args["task"].as_str().unwrap_or("");
                                tools::delegate::tool_delegate(
                                    target, task_desc,
                                    &ctx.org, &ctx.agents, &ctx.models, &ctx.global_config,
                                )
                            } else {
                                "Delegation not available in this context".to_string()
                            }
                        } else {
                            tools::execute_tool(tool_name, tool_args, &allowed_dirs)
                        };

                        // Log file-modifying operations
                        if let (Some(name), Some(dir)) = (agent_name, agents_dir) {
                            if tool_name == "write_file" {
                                let args: Value = serde_json::from_str(tool_args).unwrap_or_default();
                                if let Some(path) = args["path"].as_str() {
                                    activity::append_log(dir, name, "write_file", path);
                                }
                            } else if tool_name == "exec" {
                                let args: Value = serde_json::from_str(tool_args).unwrap_or_default();
                                if let Some(cmd) = args["command"].as_str() {
                                    activity::append_log(dir, name, "exec", cmd);
                                }
                            }
                        }

                        // Send "done" event
                        let output_summary = if output.len() > 500 {
                            format!("{}... (truncated)", &output[..500])
                        } else {
                            output.clone()
                        };

                        send_sse(
                            &tx,
                            "step",
                            &json!({
                                "name": tool_name,
                                "input": tool_args,
                                "output": output_summary,
                                "status": "done"
                            })
                            .to_string(),
                        );

                        // Add tool result to messages for LLM
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": output
                        }));
                    }
                }

                continue;
            }
        }

        // No tool calls — final reply
        let reply = msg["content"].as_str().unwrap_or("(empty)").to_string();
        send_sse(&tx, "done", &json!({"reply": reply}).to_string());
        *reply_out.lock().unwrap() = Some(reply);
        return;
    }

    send_sse(
        &tx,
        "error",
        &json!({"error": "Exceeded 10 tool calling rounds"}).to_string(),
    );
}

// ===== Legacy Config API =====

pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<Config> {
    let cfg = state.config.lock().await;
    Json(cfg.clone())
}

pub async fn post_config(
    State(state): State<Arc<AppState>>,
    Json(new_cfg): Json<Config>,
) -> Json<Value> {
    *state.config.lock().await = new_cfg.clone();
    let _ = fs::write(
        &state.config_path,
        serde_json::to_string_pretty(&new_cfg).unwrap(),
    );
    Json(json!({}))
}

// ===== Agent API =====

pub async fn list_agents(State(state): State<Arc<AppState>>) -> Json<Value> {
    let agents = state.agents.lock().await;
    let list: Vec<Value> = agents
        .values()
        .map(|a| {
            json!({
                "name": a.name,
                "model": a.model,
                "tools": a.tools,
                "model_name": a.model_name,
            })
        })
        .collect();
    Json(json!({"agents": list}))
}

pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(cfg): Json<AgentConfig>,
) -> Json<Value> {
    let dir = state.agents_dir.clone();

    match agents::create_agent(&dir, &cfg) {
        Ok(()) => {
            state
                .agents
                .lock()
                .await
                .insert(cfg.name.clone(), cfg.clone());
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Json<Value> {
    let dir = state.agents_dir.clone();

    match agents::delete_agent(&dir, &name) {
        Ok(()) => {
            state.agents.lock().await.remove(&name);
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

pub async fn get_agent_config(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Json<Value> {
    let agents = state.agents.lock().await;
    match agents.get(&name) {
        Some(cfg) => Json(json!(cfg)),
        None => Json(json!({"error": "Agent not found"})),
    }
}

pub async fn put_agent_config(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Json(cfg): Json<AgentConfig>,
) -> Json<Value> {
    let dir = state.agents_dir.clone();

    match agents::update_agent_config(&dir, &cfg) {
        Ok(()) => {
            state.agents.lock().await.insert(name, cfg);
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

// ===== Agent Chat =====

pub async fn agent_chat_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Json(req): Json<Value>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    // Look up agent config
    let agent_cfg = {
        let agents = state.agents.lock().await;
        agents.get(&name).cloned()
    };

    let agent_cfg = match agent_cfg {
        Some(c) => c,
        None => {
            send_sse(&tx, "error", &json!({"error": "Agent not found"}).to_string());
            return Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default());
        }
    };

    // Resolve LLM params from models list
    let (api_url, api_key, model) = {
        let global_config = state.config.lock().await;
        match global_config.resolve_model(agent_cfg.model_name.as_deref()) {
            Some(entry) => (entry.api_url.clone(), entry.api_key.clone(), entry.model.clone()),
            None => {
                send_sse(&tx, "error", &json!({"error": "No model configured. Add a model in Settings."}).to_string());
                return Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default());
            }
        }
    };

    // Get allowed dirs for this agent (pre-resolve canonical paths)
    let allowed_dirs = {
        let access = state.workspace_access.lock().await;
        let workspace_dir = state.workspace_dir.clone();
        let raw_dirs: Vec<PathBuf> = workspace::get_agent_projects(&access, &agent_cfg.name)
            .into_iter()
            .map(|p| workspace_dir.join(&p))
            .collect();
        tools::resolve_allowed_dirs(&raw_dirs)
    };

    let messages: Vec<Value> = req["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let tools_list = agent_cfg.tools.clone();

    // Check if agent has a position in the org chart
    let (system_prompt, tools_json) = {
        let org_chart = state.org.lock().await;
        let index = org::build_agent_position_index(&org_chart);

        if let Some(position) = index.get(&agent_cfg.name) {
            // Agent has a position — build combined system prompt
            let sp = org::build_system_prompt(&org_chart, &position, &agent_cfg);

            // Add delegate tool if position has subordinates
            let subs = org::get_subordinates(&org_chart, &position.id);
            let mut tools_value = tools::get_tools_json(&tools_list);
            if !subs.is_empty() {
                let sub_names: Vec<String> = subs.iter().flat_map(|s| s.agents.clone()).collect();
                let sub_display = if sub_names.is_empty() {
                    subs.iter().map(|s| s.title.as_str()).collect::<Vec<_>>().join("、")
                } else {
                    sub_names.join("、")
                };
                let arr = tools_value.as_array_mut().unwrap();
                arr.push(json!({
                    "type": "function",
                    "function": {
                        "name": "delegate",
                        "description": format!("将任务委派给下属。你的下属有：{}。", sub_display),
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "agent_name": { "type": "string", "description": "下属agent名称" },
                                "task": { "type": "string", "description": "要委派的任务描述" }
                            },
                            "required": ["agent_name", "task"]
                        }
                    }
                }));
                tools_value = Value::Array(arr.clone());
            }

            (Some(sp), tools_value)
        } else {
            // No position — use agent's own system_prompt
            let sp = if agent_cfg.system_prompt.is_empty() {
                None
            } else {
                Some(agent_cfg.system_prompt.clone())
            };
            (sp, tools::get_tools_json(&tools_list))
        }
    };

    // Extract last user message for history persistence
    let user_msg = messages
        .iter()
        .rev()
        .find(|m| m["role"].as_str() == Some("user"))
        .and_then(|m| m["content"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let agents_dir = state.agents_dir.clone();
    let reply_out = Arc::new(StdMutex::new(None::<String>));

    // Build delegate context (org + agents + models for delegation tool)
    let delegate_ctx = {
        let org_chart = state.org.lock().await;
        let agents_map = state.agents.lock().await;
        let global_config = state.config.lock().await;
        Some(DelegateContext {
            org: org_chart.clone(),
            agents: agents_map.clone(),
            models: global_config.models.clone(),
            global_config: global_config.clone(),
        })
    };

    tokio::task::spawn_blocking(move || {
        chat_with_tools_sse(&api_url, &api_key, &model, tools_json, allowed_dirs, messages, system_prompt, tx, reply_out.clone(), delegate_ctx, Some(&name), Some(&agents_dir));
        // Persist user message + assistant reply (save even on error)
        let reply = reply_out.lock().unwrap().take().unwrap_or_else(|| "(error)".to_string());
        history::append_exchange(&agents_dir, &name, &user_msg, &reply);
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

// ===== Legacy Chat (uses global config, no sandbox) =====

pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<Value>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    let (api_url, api_key, model) = {
        let guard = state.config.lock().await;
        match guard.resolve_model(None) {
            Some(entry) => (entry.api_url.clone(), entry.api_key.clone(), entry.model.clone()),
            None => {
                send_sse(&tx, "error", &json!({"error": "No model configured. Add a model in Settings."}).to_string());
                return Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default());
            }
        }
    };

    let messages: Vec<Value> = req["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let reply_out = Arc::new(StdMutex::new(None::<String>));
    tokio::task::spawn_blocking(move || {
        chat_with_tools_sse(&api_url, &api_key, &model, json!([]), vec![], messages, None, tx, reply_out, None, None, None);
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

// ===== Activity Log API =====

pub async fn get_logs(State(state): State<Arc<AppState>>) -> Json<Value> {
    let entries = activity::load_logs(&state.agents_dir);
    Json(json!({"logs": entries}))
}

// ===== Workspace API =====

pub async fn list_projects(State(state): State<Arc<AppState>>) -> Json<Value> {
    let projects = workspace::list_projects(&state.workspace_dir);
    Json(json!({"projects": projects}))
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<Value>,
) -> Json<Value> {
    let name = req["name"].as_str().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Json(json!({"error": "name is required"}));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Json(json!({"error": "invalid project name"}));
    }
    let dir = state.workspace_dir.join(&name);
    if dir.exists() {
        return Json(json!({"error": "project already exists"}));
    }
    match fs::create_dir_all(&dir) {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

pub async fn open_project(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Json<Value> {
    let dir = state.workspace_dir.join(&name);
    if !dir.is_dir() {
        return Json(json!({"error": "project not found"}));
    }
    let path_str = dir.to_string_lossy().to_string();
    let result = std::process::Command::new("xdg-open")
        .arg(&path_str)
        .spawn();
    match result {
        Ok(_) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

pub async fn get_access(State(state): State<Arc<AppState>>) -> Json<WorkspaceAccess> {
    let access = state.workspace_access.lock().await;
    Json(access.clone())
}

pub async fn put_access(
    State(state): State<Arc<AppState>>,
    Json(new_access): Json<WorkspaceAccess>,
) -> Json<Value> {
    let access_path = state.workspace_dir.join("access.json");
    match workspace::save_access(&access_path, &new_access) {
        Ok(()) => {
            *state.workspace_access.lock().await = new_access;
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

// ===== Org API =====

pub async fn get_org(State(state): State<Arc<AppState>>) -> Json<OrgChart> {
    let org = state.org.lock().await;
    Json(org.clone())
}

pub async fn create_position(
    State(state): State<Arc<AppState>>,
    Json(req): Json<Value>,
) -> Json<Value> {
    let title = req["title"].as_str().unwrap_or("").to_string();
    if title.is_empty() {
        return Json(json!({"error": "title is required"}));
    }
    let parent_id = req["parent_id"].as_str().map(|s| s.to_string());

    let create_agent_data: Option<AgentConfig> = req
        .get("create_agent")
        .and_then(|v| v.as_object())
        .map(|obj| AgentConfig {
            name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            model: obj.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            api_url: obj.get("api_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            api_key: obj.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            system_prompt: obj.get("system_prompt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            tools: obj.get("tools").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            model_name: obj.get("model_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        });

    let mut org = state.org.lock().await;

    // Create agent if requested
    let new_agent_name = if let Some(ref agent_cfg) = create_agent_data {
        match agents::create_agent(&state.agents_dir, agent_cfg) {
            Ok(()) => {
                state.agents.lock().await.insert(agent_cfg.name.clone(), agent_cfg.clone());
                Some(agent_cfg.name.clone())
            }
            Err(e) => return Json(json!({"error": format!("Failed to create agent: {}", e)})),
        }
    } else {
        None
    };

    // Add position with agent assigned if created
    let mut pos = org::add_position(&mut org, title, parent_id);
    if let Some(ref name) = new_agent_name {
        if let Some(p) = org.positions.iter_mut().find(|p| p.id == pos.id) {
            p.agents.push(name.clone());
            pos = p.clone();
        }
    }

    let _ = org::save_org(&state.org_path, &org);
    Json(json!({ "position": pos, "agent_created": new_agent_name }))
}

pub async fn update_position_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Json(req): Json<Value>,
) -> Json<Value> {
    let title = req["title"].as_str().map(|s| s.to_string());
    let parent_id = req.get("parent_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let agents = req["agents"].as_array().map(|a| {
        a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
    });
    let system_prompt = req["system_prompt"].as_str().map(|s| s.to_string());

    let mut org = state.org.lock().await;
    // parent_id needs to handle null vs not provided
    let parent_id_opt = if req.get("parent_id").is_some() {
        Some(parent_id)
    } else {
        None
    };

    match org::update_position(&mut org, &id, title, parent_id_opt, agents, system_prompt) {
        Ok(()) => {
            let _ = org::save_org(&state.org_path, &org);
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

pub async fn delete_position(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Json<Value> {
    let mut org = state.org.lock().await;
    match org::remove_position(&mut org, &id) {
        Ok(()) => {
            let _ = org::save_org(&state.org_path, &org);
            Json(json!({"ok": true}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

// ===== History API =====

pub async fn get_agent_history(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Json<Value> {
    let entries = history::load_history(&state.agents_dir, &name);
    Json(json!({"messages": entries}))
}

pub async fn clear_agent_history(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Json<Value> {
    match history::clear_history(&state.agents_dir, &name) {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e})),
    }
}

// ===== Task Handlers =====

pub async fn get_tasks(State(state): State<Arc<AppState>>) -> Json<Value> {
    let board = state.tasks.lock().await;
    Json(json!({ "tasks": board.tasks }))
}

#[derive(Deserialize)]
pub struct CreateTaskBody {
    pub title: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateTaskBody>,
) -> Json<Value> {
    if body.title.trim().is_empty() {
        return Json(json!({"error": "Title is required"}));
    }
    let mut board = state.tasks.lock().await;
    let task = tasks::add_task(&mut board, body.title, body.description);
    tasks::save_tasks(&state.tasks_path, &board);
    Json(json!({"ok": true, "task": task}))
}

#[derive(Deserialize)]
pub struct UpdateTaskBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub agents: Option<Vec<String>>,
    pub progress: Option<u32>,
}

pub async fn update_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<UpdateTaskBody>,
) -> Json<Value> {
    let mut board = state.tasks.lock().await;
    if tasks::update_task(&mut board, &id, body.title, body.description, body.status, body.agents, body.progress) {
        tasks::save_tasks(&state.tasks_path, &board);
        Json(json!({"ok": true}))
    } else {
        Json(json!({"error": "Task not found"}))
    }
}

pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Json<Value> {
    let mut board = state.tasks.lock().await;
    if tasks::delete_task(&mut board, &id) {
        tasks::save_tasks(&state.tasks_path, &board);
        Json(json!({"ok": true}))
    } else {
        Json(json!({"error": "Task not found"}))
    }
}
