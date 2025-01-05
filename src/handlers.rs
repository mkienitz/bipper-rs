use crate::crypto::{mnemonic_to_hash, restore_filename, DecryptionIter, Encryptor};
use crate::database::Database;
use crate::errors::AppError;
use axum::body::Body;
use axum::{
    extract::{Json, Path, State},
    http::header,
    response::IntoResponse,
};
use futures_util::{stream, StreamExt};
use serde::Deserialize;
use tokio::fs;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

#[derive(Deserialize)]
pub struct AccessInfo {
    mnemonic: String,
}

pub async fn retrieve_handler(
    State(state): State<AppState>,
    Json(access_info): Json<AccessInfo>,
) -> Result<impl IntoResponse, AppError> {
    let entropy_hash = mnemonic_to_hash(&access_info.mnemonic)?;
    let metadata = state.db.find_blob(&entropy_hash).await?;
    let filename = restore_filename(
        &access_info.mnemonic,
        metadata.filename_cipher,
        metadata.filename_nonce,
    )?;

    let storage_path = format!("store/{}", entropy_hash);
    let decryption_iter = DecryptionIter::new(&storage_path, &access_info.mnemonic)?;

    let decryption_stream = stream::iter(decryption_iter);
    let stream_body = Body::from_stream(decryption_stream);

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename={}", filename),
        ),
    ];

    Ok((headers, stream_body))
}

pub async fn store_handler(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    body: Body,
) -> Result<impl IntoResponse, AppError> {
    let mut encryptor = Encryptor::new(&filename).await?;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        encryptor.update(&chunk?).await?;
    }
    let (mnemonic, metadata) = encryptor.finalize().await?;
    state.db.insert_blob(&metadata).await?;
    Ok(mnemonic)
}

pub async fn delete_handler(
    State(state): State<AppState>,
    Json(access_info): Json<AccessInfo>,
) -> Result<impl IntoResponse, AppError> {
    let passphrase_hash = mnemonic_to_hash(&access_info.mnemonic)?;
    state.db.delete_blob(&passphrase_hash).await?;
    fs::remove_file(format!("store/{}", passphrase_hash)).await?;
    Ok("File successfully deleted!")
}
