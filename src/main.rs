use anyhow::{Error, Result};
use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;
use bipper::database::Database;
use bipper::handlers::{delete_handler, retrieve_handler, store_handler, AppState};
use std::net::SocketAddr;
use std::str::FromStr;
use std::{env, fs};
use dotenv::dotenv;
use tracing::info;

async fn setup() -> Result<AppState> {
    dotenv().ok();
    let db_host = env::var("BIPPER_POSTGRES_HOST");
    let db_port = env::var("BIPPER_POSTGRES_PORT");
    let db_user = env::var("BIPPER_POSTGRES_USER");
    let db_database = env::var("BIPPER_POSTGRES_DATABASE");
    let db_password = env::var("BIPPER_POSTGRES_PASSWORD");
    info!(?db_host, ?db_port, ?db_user, ?db_database, ?db_password);

    let database_url = match (db_host, db_port, db_user, db_database, db_password) {
        (Ok(host), Ok(port), Ok(user), Ok(db), Ok(pass)) => {
            format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db)
        }
        (Ok(host), Err(_), Ok(user), Ok(db), Err(_)) => {
            format!("postgres://{}@localhost/{}?host={}", user, db, host)
        }
        _ => return Err(Error::msg("Invalid environment!")),
    };

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
    let bipper_addr = env::var("BIPPER_ADDRESS")?;
    let bipper_port = env::var("BIPPER_PORT")?;
    let addr = SocketAddr::from_str(&format!("{}:{}", bipper_addr, bipper_port))?;
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
