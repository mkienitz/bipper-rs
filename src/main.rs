use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;
use bipper::database::Database;
use bipper::handlers::{delete_handler, retrieve_handler, store_handler, AppState};
use dotenv::dotenv;
use std::net::SocketAddr;
use std::{env, fs};

async fn setup() -> Result<AppState> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let state = AppState {
        db: Database::new(&database_url).await?,
    };
    state.db.create_tables().await?;
    fs::create_dir_all("store")?;
    Ok(state)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let state = setup().await?;
    let app = Router::new()
        .route("/store/:filename", post(store_handler))
        .route("/retrieve", post(retrieve_handler))
        .route("/delete", post(delete_handler))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
