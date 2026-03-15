use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use miner_core::coin::supported_coins;
use miner_core::config::MinerConfig;
use miner_core::stats::StatsSnapshot;
use miner_worker::manager::WorkerManager;

#[derive(Parser)]
#[command(name = "miner", about = "Multi-cryptocurrency mining engine")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,

    /// Generate a default config file and exit
    #[arg(long)]
    init_config: bool,

    /// Override log level
    #[arg(short, long)]
    log_level: Option<String>,
}

type AppState = Arc<RwLock<WorkerManager>>;

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

    let manager = Arc::new(RwLock::new(WorkerManager::new()));

    // Start workers for each enabled coin
    {
        let mut mgr = manager.write().await;
        for coin in &config.coins {
            if coin.enabled {
                match mgr.start_worker(coin).await {
                    Ok(id) => info!(worker_id = %id, coin = %coin.symbol, "Worker started"),
                    Err(e) => tracing::error!(coin = %coin.symbol, error = %e, "Failed to start worker"),
                }
            }
        }
    }

    if config.api.enabled {
        let addr = format!("{}:{}", config.api.host, config.api.port);
        info!(addr = %addr, "Starting API server");

        let app = api_router(manager.clone());

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    } else {
        info!("API server disabled, running headless");
        tokio::signal::ctrl_c().await?;
    }

    info!("Shutting down...");
    manager.write().await.stop_all().await;

    Ok(())
}

fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/stats", get(get_stats))
        .route("/api/coins", get(get_coins))
        .route("/api/workers/stop-all", post(stop_all_workers))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn get_stats(State(manager): State<AppState>) -> Json<Vec<StatsSnapshot>> {
    let mgr = manager.read().await;
    Json(mgr.get_stats().await)
}

async fn get_coins(
) -> Json<Vec<miner_core::coin::Coin>> {
    Json(supported_coins())
}

async fn stop_all_workers(State(manager): State<AppState>) -> StatusCode {
    let mut mgr = manager.write().await;
    mgr.stop_all().await;
    StatusCode::OK
}
