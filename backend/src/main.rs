mod agents;
mod activity;
mod config;
mod handlers;
mod history;
mod org;
mod tasks;
mod tools;
mod workspace;

use std::fs;
use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use config::{AppState, Config};

#[tokio::main]
async fn main() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to resolve project root")
        .to_path_buf();

    // Legacy global config
    let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.json");
    let config: Config = fs::read_to_string(&config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Ensure directories exist
    let agents_dir = project_root.join("agents");
    let workspace_dir = project_root.join("workspace");
    let _ = fs::create_dir_all(&agents_dir);
    let _ = fs::create_dir_all(&workspace_dir);

    // Load agents from disk
    let agents_map = agents::load_agents(&agents_dir);
    println!(
        "Loaded {} agent(s): {}",
        agents_map.len(),
        agents_map
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Load workspace access control
    let access_path = workspace_dir.join("access.json");
    let workspace_access = workspace::load_access(&access_path);

    // Load org chart
    let org_path = agents_dir.join("org.json");
    let org_chart = org::load_org(&org_path);

    // Load task board
    let tasks_path = agents_dir.join("tasks.json");
    let task_board = tasks::load_tasks(&tasks_path);

    let state = Arc::new(AppState {
        config: Arc::new(tokio::sync::Mutex::new(config)),
        config_path,
        agents: Arc::new(tokio::sync::Mutex::new(agents_map)),
        workspace_access: Arc::new(tokio::sync::Mutex::new(workspace_access)),
        agents_dir,
        workspace_dir,
        org: Arc::new(tokio::sync::Mutex::new(org_chart)),
        org_path,
        tasks: Arc::new(tokio::sync::Mutex::new(task_board)),
        tasks_path,
    });

    let frontend_dir = project_root.join("frontend");

    let app = Router::new()
        // Legacy config API
        .route("/api/config", get(handlers::get_config).post(handlers::post_config))
        // Legacy chat (global config, no sandbox)
        .route("/api/chat", post(handlers::chat_handler))
        // Agent API
        .route("/api/agents", get(handlers::list_agents).post(handlers::create_agent))
        .route(
            "/api/agents/:name",
            delete(handlers::delete_agent),
        )
        .route(
            "/api/agents/:name/config",
            get(handlers::get_agent_config).put(handlers::put_agent_config),
        )
        .route("/api/agents/:name/chat", post(handlers::agent_chat_handler))
        .route("/api/agents/:name/history", get(handlers::get_agent_history).delete(handlers::clear_agent_history))
        // Activity Log
        .route("/api/logs", get(handlers::get_logs))
        // Workspace API
        .route("/api/workspace/projects", get(handlers::list_projects).post(handlers::create_project))
        .route(
            "/api/workspace/projects/:name/open",
            post(handlers::open_project),
        )
        .route("/api/workspace/access", get(handlers::get_access).put(handlers::put_access))
        // Org API
        .route("/api/org", get(handlers::get_org))
        .route("/api/org/positions", post(handlers::create_position))
        .route(
            "/api/org/positions/:id",
            put(handlers::update_position_handler).delete(handlers::delete_position),
        )
        // Tasks API
        .route("/api/tasks", get(handlers::get_tasks).post(handlers::create_task))
        .route(
            "/api/tasks/:id",
            put(handlers::update_task).delete(handlers::delete_task),
        )
        // Static frontend
        .fallback_service(
            ServeDir::new(&frontend_dir)
                .fallback(ServeFile::new(frontend_dir.join("index.html"))),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8181")
        .await
        .expect("Failed to bind port 8181");

    println!("HTTP server started: http://127.0.0.1:8181");

    if let Err(e) = webbrowser::open("http://127.0.0.1:8181") {
        eprintln!("Failed to open browser: {e}");
        println!("Please open http://127.0.0.1:8181 manually.");
    }

    axum::serve(listener, app).await.expect("Server error");
}
