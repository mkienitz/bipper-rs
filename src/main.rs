use anyhow::Result;
use dotenv::dotenv;
use bipper::database::Database;
use bipper::handlers::{homepage_handler, retrieve_handler, store_handler, AppState, State};
use std::sync::Arc;
use std::{env, fs};

async fn setup() -> Result<State> {
    femme::start();
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let state: State = Arc::new(AppState {
        db: Database::new(&database_url).await?,
    });
    fs::create_dir_all("store")?;
    Ok(state)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let state = setup().await?;
    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());
    app.at("/").get(homepage_handler);
    app.at("/store/:file").post(store_handler);
    app.at("/retrieve").post(retrieve_handler);
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
