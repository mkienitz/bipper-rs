use crate::crypto::{calculate_passphrase_hash, decrypt_file, encrypt_file};
use crate::database::Database;
use anyhow::anyhow;
use axum::{
    extract::{Multipart, State},
    http::header::{self, HeaderMap},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
};
use tracing::log::trace;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        trace!("{}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
pub struct RetrievalForm {
    mnemonic: String,
}

pub async fn retrieve_handler(
    State(state): State<AppState>,
    Form(retrieval_form): Form<RetrievalForm>,
) -> Result<impl IntoResponse, AppError> {
    let passphrase_hash = calculate_passphrase_hash(&retrieval_form.mnemonic).await?;
    let metadata = state.db.find_blob(&passphrase_hash).await?;

    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(format!("store/{}", metadata.filename))?;
    let mut content_bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut content_bytes)?;

    let filename = decrypt_file(&mut content_bytes, &retrieval_form.mnemonic, &metadata).await?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse()?);
    headers.insert(
        header::CONTENT_DISPOSITION,
        (&format!("attachment; filename={}", filename)).parse()?,
    );

    Ok((headers, content_bytes))
}

pub async fn store_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    if let Some(field) = multipart.next_field().await? {
        let filename = field
            .file_name()
            .ok_or(anyhow!("Field contains no filename!"))?
            .to_owned();
        let mut bytes = field.bytes().await?.into();
        let (mnemonic, metadata) = encrypt_file(&mut bytes, &filename).await?;
        state.db.insert_blob(&metadata).await?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("store/{}", metadata.filename))?;
        file.write_all(&bytes)?;
        Ok(mnemonic)
    } else {
        Err(anyhow!("No field found!").into())
    }
}

pub async fn homepage_handler() -> impl IntoResponse  {
    Html(include_str!("index.html"))
}
