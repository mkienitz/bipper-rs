use crate::crypto::{mnemonic_to_hash, restore_filename, DecryptionIter, Encryptor};
use crate::database::Database;
use crate::errors::AppError;
use axum::body::StreamBody;
use axum::{
    extract::{BodyStream, Json, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::{stream, StreamExt};
use serde::Deserialize;
use tokio::fs;
use tracing::debug;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
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
    debug!(storage_path);
    let decryption_iter = DecryptionIter::new(&storage_path, &access_info.mnemonic)?;

    let body_stream = stream::iter(decryption_iter);
    let body = StreamBody::new(body_stream);

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename={}", filename),
        ),
    ];

    Ok((headers, body))
}

pub async fn store_handler(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, AppError> {
    let mut enc_state = Encryptor::new(&filename).await?;
    while let Some(chunk) = stream.next().await {
        enc_state.update(&chunk?).await?;
    }
    let (mnemonic, metadata) = enc_state.finalize().await?;
    state.db.insert_blob(&metadata).await?;
    let headers = [(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")];
    Ok((headers, mnemonic))
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
