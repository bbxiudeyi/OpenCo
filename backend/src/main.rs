mod agents;
mod activity;
mod config;
mod fix;
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
use clap::{Parser, Subcommand};
use tower_http::services::{ServeDir, ServeFile};

use config::{AppState, Config};

#[derive(Parser)]
#[command(name = "openco", version, about = "AI agent collaboration platform")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on
    #[arg(long, default_value = "8181")]
    port: u16,

    /// Data directory (default: ~/.openco)
    #[arg(long)]
    data_dir: Option<String>,

    /// Don't open browser automatically
    #[arg(long)]
    no_browser: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Check and repair data files + installation integrity
    Fix {
        /// Data directory to fix (default: ~/.openco)
        #[arg(long)]
        data_dir: Option<String>,
    },
}

fn resolve_data_dir(cli_data_dir: Option<&str>) -> std::path::PathBuf {
    if let Some(dir) = cli_data_dir {
        std::path::PathBuf::from(dir)
    } else {
        dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".openco")
    }
}

fn resolve_frontend_dir() -> std::path::PathBuf {
    // First: try relative to the executable (npm distribution)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let frontend = exe_dir.join("frontend");
            if frontend.join("index.html").exists() {
                return frontend;
            }
        }
    }

    // Fallback: development mode, relative to CARGO_MANIFEST_DIR
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to resolve project root")
        .join("frontend")
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Fix { data_dir }) => {
            let data_dir = resolve_data_dir(data_dir.as_deref());
            fix::run_fix(&data_dir);
            return;
        }
        None => {}
    }

    let data_dir = resolve_data_dir(
        cli.data_dir
            .as_deref()
            .or(cli.command.as_ref().and_then(|c| match c {
                Commands::Fix { data_dir } => data_dir.as_deref(),
            })),
    );

    // Ensure data directories exist
    let agents_dir = data_dir.join("agents");
    let workspace_dir = data_dir.join("workspace");
    let _ = fs::create_dir_all(&agents_dir);
    let _ = fs::create_dir_all(&workspace_dir);

    // Load config from data dir
    let config_path = data_dir.join("config.json");
    let config: Config = fs::read_to_string(&config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Write default config if it doesn't exist
    if !config_path.exists() {
        if let Ok(json) = serde_json::to_string_pretty(&Config::default()) {
            let _ = fs::write(&config_path, json);
        }
    }

    // Load agents from data dir
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

    let frontend_dir = resolve_frontend_dir();

    let app = Router::new()
        // Config API
        .route("/api/config", get(handlers::get_config).post(handlers::post_config))
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

    let addr = format!("127.0.0.1:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind {}: {}", addr, e);
            std::process::exit(1);
        });

    println!("OpenCo server started: http://{}", addr);
    println!("Data directory: {}", data_dir.display());

    if !cli.no_browser {
        if let Err(e) = webbrowser::open(&format!("http://{}", addr)) {
            eprintln!("Failed to open browser: {e}");
            println!("Please open http://{} manually.", addr);
        }
    }

    axum::serve(listener, app).await.expect("Server error");
}
