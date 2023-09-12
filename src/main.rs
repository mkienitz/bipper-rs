use anyhow::Result;
use dotenv::dotenv;
use rbipper::models::BlobMetadata;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::io::{Read, Write};
use std::sync::Arc;
use std::{env, fs};
use tide::http::mime;
use tide::{self, Body, Request, Response};

use rbipper::crypto::{calculate_passphrase_hash, decrypt_file, encrypt_file};

struct AppState {
    conn: PgPool,
}

impl AppState {
    async fn new(database_url: &str) -> Result<AppState> {
        let conn = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await?;
        Ok(AppState { conn })
    }
}

type State = Arc<AppState>;

async fn setup(state: &State) -> Result<()> {
    fs::create_dir_all("store")?;
    sqlx::query!(
        r#"CREATE TABLE IF NOT EXISTS blobs (
            passphrase_hash BYTEA NOT NULL PRIMARY KEY,
            filename VARCHAR(255) NOT NULL,
            content_nonce BYTEA NOT NULL,
            filename_nonce BYTEA NOT NULL
        )"#
    )
    .execute(&state.conn)
    .await?;
    Ok(())
}

async fn retrieve_handler(mut req: Request<State>) -> tide::Result {
    #[derive(Deserialize, Serialize)]
    struct BodyParam {
        mnemonic: String,
    }
    let BodyParam { mnemonic } = req.body_form().await?;
    let passphrase_hash = calculate_passphrase_hash(&mnemonic).await?;

    let metadata = sqlx::query_as!(
        BlobMetadata,
        r#"SELECT * FROM blobs
            WHERE passphrase_hash = $1"#,
        passphrase_hash
    )
    .fetch_one(&req.state().conn)
    .await?;

    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(format!("store/{}", metadata.filename))?;
    let mut content_bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut content_bytes)?;

    let filename = decrypt_file(&mut content_bytes, &mnemonic, &metadata).await?;

    let response = Response::builder(200)
        .body(Body::from_bytes(content_bytes))
        .header(
            "Content-Disposition",
            format!("attachment; filename={}", filename),
        )
        .content_type(mime::BYTE_STREAM)
        .build();
    Ok(response)
}

async fn store_handler(mut req: Request<State>) -> tide::Result {
    let mut bytes = req.body_bytes().await?;
    let file_name = req.param("file")?;
    tide::log::info!("File found!");
    let (mnemonic, metadata) = encrypt_file(&mut bytes, file_name).await?;
    tide::log::info!("Encryption succeeded!");
    sqlx::query_as!(
        BlobMetadata,
        r#"INSERT INTO blobs
                    (passphrase_hash, filename, content_nonce, filename_nonce)
                    VALUES ($1, $2, $3, $4)"#,
        metadata.passphrase_hash,
        metadata.filename,
        metadata.content_nonce,
        metadata.filename_nonce
    )
    .execute(&req.state().conn)
    .await?;
    tide::log::info!("Insertion into DB succeeded!");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("store/{}", metadata.filename))?;
    file.write_all(&bytes)?;

    tide::log::info!("File written!");
    Ok(mnemonic.into())
}

async fn homepage_handler(_req: Request<State>) -> tide::Result {
    let response = Response::builder(200)
        .body(
            r#"
            <html>
                <head>
                    <title>Bipper</title>
                </head>
                <body>
                    <form action="/retrieve" method="post" target="_blank">
                        <label for="words">BIP39 words:</label><br>
                        <input type="text" id="words" name="mnemonic" value=""><br>
                        <input type="submit" value="Submit">
                    </form>
                </body>
            </html>"#,
        )
        .content_type(mime::HTML)
        .build();
    Ok(response)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    // Load environment
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    // Use femme for pretty logs
    femme::start();
    let state: State = Arc::new(AppState::new(&database_url).await?);
    setup(&state).await?;
    // Start tide
    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());
    app.at("/store/:file").post(store_handler);
    app.at("/retrieve").post(retrieve_handler);
    app.at("/").get(homepage_handler);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
