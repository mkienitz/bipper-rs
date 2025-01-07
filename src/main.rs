use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;
use bipper::database::Database;
use bipper::handlers::{delete_handler, retrieve_handler, store_handler, AppState};
use std::net::SocketAddr;
use std::str::FromStr;
use std::{env, fs};
use tokio::net::TcpListener;
use tracing::debug;

async fn setup() -> Result<AppState> {
    let state = AppState {
        db: Database::new().await?,
    };
    fs::create_dir_all("store")?;
    Ok(state)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let state = setup().await?;
    let router = Router::new()
        .route("/store/{filename}", post(store_handler))
        .route("/retrieve", post(retrieve_handler))
        .route("/delete", post(delete_handler))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .with_state(state);

    let bipper_addr = env::var("BIPPER_ADDRESS")?;
    let bipper_port = env::var("BIPPER_PORT")?;
    let addr = SocketAddr::from_str(&format!("{}:{}", bipper_addr, bipper_port))?;
    let listener = TcpListener::bind(addr).await?;

    debug!("listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
