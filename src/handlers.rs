use crate::crypto::{calculate_passphrase_hash, DecryptionState, EncryptionState};
use crate::database::Database;
use axum::body::StreamBody;
use axum::{
    extract::{BodyStream, Json, Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use futures_util::{stream, StreamExt};
use serde::Deserialize;
use tokio::fs;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
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
    let passphrase_hash = calculate_passphrase_hash(&access_info.mnemonic).await?;
    let metadata = state.db.find_blob(&passphrase_hash).await?;

    let storage_path = format!("store/{}", hex::encode(passphrase_hash));
    let decryption_state =
        DecryptionState::new(storage_path, &access_info.mnemonic, metadata).await?;
    let filename = decryption_state.filename()?;

    let body_stream = stream::iter(decryption_state);
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
    let mut enc_state = EncryptionState::new(&filename).await?;
    while let Some(chunk) = stream.next().await {
        enc_state.update(&chunk?).await?;
    }
    let (mnemonic, metadata) = enc_state.finalize().await?;
    state.db.insert_blob(&metadata).await?;
    Ok(mnemonic)
}

pub async fn delete_handler(
    State(state): State<AppState>,
    Json(access_info): Json<AccessInfo>,
) -> Result<impl IntoResponse, AppError> {
    let passphrase_hash = calculate_passphrase_hash(&access_info.mnemonic).await?;
    state.db.delete_blob(&passphrase_hash).await?;
    fs::remove_file(format!("store/{}", hex::encode(passphrase_hash))).await?;
    Ok("File successfully deleted!")
}

pub async fn homepage_handler() -> impl IntoResponse {
    Html(include_str!("index.html"))
}
