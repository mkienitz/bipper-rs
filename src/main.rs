use anyhow::Result;
use dotenv::dotenv;
use rbipper::database::Database;
use rbipper::handlers::{homepage_handler, retrieve_handler, store_handler, State};
use std::sync::Arc;
use std::{env, fs};

fn setup() -> Result<()> {
    fs::create_dir_all("store")?;
    femme::start();
    Ok(())
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    setup()?;
    let state: State = Arc::new(Database::new(&database_url).await?);

    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());
    app.at("/store/:file").post(store_handler);
    app.at("/retrieve").post(retrieve_handler);
    app.at("/").get(homepage_handler);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
