use sqlx::FromRow;

#[derive(FromRow)]
pub struct BlobMetadata {
    pub entropy_hash: String,
    pub filename_cipher: Vec<u8>,
    pub filename_nonce: [u8; 12],
}
