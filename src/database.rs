use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models::BlobMetadata;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await?;
        Ok(Database { pool })
    }

    pub async fn create_tables(&self) -> Result<()> {
        Ok(sqlx::migrate!("./migrations").run(&self.pool).await?)
    }

    pub async fn insert_blob(&self, metadata: &BlobMetadata) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO blobs
                    (passphrase_hash, filename, content_nonce, filename_nonce, cipher_hash)
                    VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(&metadata.passphrase_hash)
        .bind(&metadata.filename)
        .bind(&metadata.content_nonce)
        .bind(&metadata.filename_nonce)
        .bind(&metadata.cipher_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_blob(&self, passphrase_hash: &Vec<u8>) -> Result<BlobMetadata> {
        let metadata =
            sqlx::query_as::<_, BlobMetadata>(r#"SELECT * FROM blobs WHERE passphrase_hash = $1"#)
                .bind(passphrase_hash)
                .fetch_one(&self.pool)
                .await?;
        Ok(metadata)
    }
    pub async fn delete_blob(&self, passphrase_hash: &Vec<u8>) -> Result<BlobMetadata> {
        let metadata = sqlx::query_as::<_, BlobMetadata>(
            r#"DELETE FROM blobs WHERE passphrase_hash = $1 RETURNING *"#,
        )
        .bind(passphrase_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(metadata)
    }
}
