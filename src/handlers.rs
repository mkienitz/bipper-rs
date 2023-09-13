use std::{
    fs,
    io::{Read, Write},
    sync::Arc,
};

use crate::crypto::{calculate_passphrase_hash, decrypt_file, encrypt_file};
use serde::Deserialize;
use tide::{http::mime, Body, Request, Response};

use crate::database::Database;

pub struct AppState {
    pub db: Database,
}

pub type State = Arc<AppState>;

pub async fn retrieve_handler(mut req: Request<State>) -> tide::Result {
    #[derive(Deserialize)]
    struct BodyParam {
        mnemonic: String,
    }
    let BodyParam { mnemonic } = req.body_form().await?;
    let passphrase_hash = calculate_passphrase_hash(&mnemonic).await?;

    let metadata = req.state().db.find_blob(&passphrase_hash).await?;

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

pub async fn store_handler(mut req: Request<State>) -> tide::Result {
    let mut bytes = req.body_bytes().await?;
    let file_name = req.param("file")?;
    let (mnemonic, metadata) = encrypt_file(&mut bytes, file_name).await?;

    req.state().db.insert_blob(&metadata).await?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("store/{}", metadata.filename))?;
    file.write_all(&bytes)?;

    tide::log::info!("File written!");
    Ok(mnemonic.into())
}

pub async fn homepage_handler(_req: Request<State>) -> tide::Result {
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
