use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use clap::Parser;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use miner_core::coin::supported_coins;
use miner_core::config::{CoinEntry, MinerConfig};
use miner_core::stats::StatsSnapshot;
use miner_worker::manager::WorkerManager;

#[derive(Parser)]
#[command(name = "miner", about = "Multi-cryptocurrency mining engine")]
struct Cli {
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,

    #[arg(long)]
    init_config: bool,

    #[arg(short, long)]
    log_level: Option<String>,
}

struct SharedState {
    manager: RwLock<WorkerManager>,
    config: RwLock<MinerConfig>,
    config_path: PathBuf,
}

type AppState = Arc<SharedState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        let config = MinerConfig::default();
        config.save(&cli.config)?;
        println!("Default config written to {}", cli.config.display());
        return Ok(());
    }

    let config = if cli.config.exists() {
        MinerConfig::load(&cli.config)?
    } else {
        eprintln!(
            "Config file not found: {}. Run with --init-config to create one.",
            cli.config.display()
        );
        std::process::exit(1);
    };

    let log_level = cli
        .log_level
        .unwrap_or_else(|| config.log_level.clone());

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .init();

    info!("Starting miner v{}", env!("CARGO_PKG_VERSION"));

    let mut manager = WorkerManager::new();

    for coin in &config.coins {
        if coin.enabled {
            match manager.start_worker(coin).await {
                Ok(id) => info!(worker_id = %id, coin = %coin.symbol, "Worker started"),
                Err(e) => tracing::error!(coin = %coin.symbol, error = %e, "Failed to start worker"),
            }
        }
    }

    let state: AppState = Arc::new(SharedState {
        manager: RwLock::new(manager),
        config: RwLock::new(config.clone()),
        config_path: cli.config.clone(),
    });

    if config.api.enabled {
        let addr = format!("{}:{}", config.api.host, config.api.port);
        info!(addr = %addr, "Starting API server");

        let app = api_router(state.clone());

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    } else {
        info!("API server disabled, running headless");
        tokio::signal::ctrl_c().await?;
    }

    info!("Shutting down...");
    state.manager.write().await.stop_all().await;

    Ok(())
}

fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/stats", get(get_stats))
        .route("/api/coins", get(get_coins))
        .route("/api/config", get(get_config))
        .route("/api/config/coins/{index}", put(update_coin_entry))
        .route("/api/workers/status", get(get_worker_status))
        .route("/api/workers/start/{index}", post(start_worker))
        .route("/api/workers/stop/{index}", post(stop_worker))
        .route("/api/workers/stop-all", post(stop_all_workers))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn get_stats(State(state): State<AppState>) -> Json<Vec<StatsSnapshot>> {
    let mgr = state.manager.read().await;
    Json(mgr.get_stats().await)
}

async fn get_coins() -> Json<Vec<miner_core::coin::Coin>> {
    Json(supported_coins())
}

async fn get_config(State(state): State<AppState>) -> Json<MinerConfig> {
    let cfg = state.config.read().await;
    Json(cfg.clone())
}

async fn update_coin_entry(
    State(state): State<AppState>,
    Path(index): Path<usize>,
    Json(entry): Json<CoinEntry>,
) -> Result<Json<MinerConfig>, StatusCode> {
    let mut cfg = state.config.write().await;

    if index >= cfg.coins.len() {
        return Err(StatusCode::NOT_FOUND);
    }

    cfg.coins[index] = entry;

    if let Err(e) = cfg.save(&state.config_path) {
        tracing::error!(error = %e, "Failed to save config");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Config updated for coin index {}", index);
    Ok(Json(cfg.clone()))
}

async fn get_worker_status(
    State(state): State<AppState>,
) -> Json<std::collections::HashMap<String, bool>> {
    let cfg = state.config.read().await;
    let mgr = state.manager.read().await;
    let mut status = std::collections::HashMap::new();
    for coin in &cfg.coins {
        status.insert(coin.symbol.clone(), mgr.has_worker_for_symbol(&coin.symbol));
    }
    Json(status)
}

async fn start_worker(
    State(state): State<AppState>,
    Path(index): Path<usize>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let cfg = state.config.read().await;

    if index >= cfg.coins.len() {
        return Err(StatusCode::NOT_FOUND);
    }

    let coin = &cfg.coins[index];

    if coin.wallet.is_empty()
        || coin.wallet.starts_with("YOUR_")
        || coin.pool.url.is_empty()
    {
        return Ok(Json(serde_json::json!({
            "ok": false,
            "error": "Worker is not configured (missing wallet or pool)"
        })));
    }

    let coin_clone = coin.clone();
    let symbol = coin.symbol.clone();
    drop(cfg);

    let mut mgr = state.manager.write().await;

    if mgr.has_worker_for_symbol(&symbol) {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "message": "Worker already running"
        })));
    }

    match mgr.start_worker(&coin_clone).await {
        Ok(id) => {
            info!(worker_id = %id, coin = %symbol, "Worker started via API");
            Ok(Json(serde_json::json!({
                "ok": true,
                "worker_id": id
            })))
        }
        Err(e) => {
            tracing::error!(coin = %symbol, error = %e, "Failed to start worker via API");
            Ok(Json(serde_json::json!({
                "ok": false,
                "error": format!("{}", e)
            })))
        }
    }
}

async fn stop_worker(
    State(state): State<AppState>,
    Path(index): Path<usize>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let cfg = state.config.read().await;

    if index >= cfg.coins.len() {
        return Err(StatusCode::NOT_FOUND);
    }

    let symbol = cfg.coins[index].symbol.clone();
    drop(cfg);

    let mut mgr = state.manager.write().await;
    let stopped = mgr.stop_worker_by_symbol(&symbol).await;

    info!(coin = %symbol, stopped, "Worker stop requested via API");
    Ok(Json(serde_json::json!({
        "ok": true,
        "stopped": stopped
    })))
}

async fn stop_all_workers(State(state): State<AppState>) -> StatusCode {
    let mut mgr = state.manager.write().await;
    mgr.stop_all().await;
    StatusCode::OK
}
