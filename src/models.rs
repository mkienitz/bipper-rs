use sqlx::FromRow;

#[derive(FromRow)]
pub struct BlobMetadata {
    pub passphrase_hash: Vec<u8>,
    pub filename: String,
    pub content_nonce: Vec<u8>,
    pub filename_nonce: Vec<u8>,
}
