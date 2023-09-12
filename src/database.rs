use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models::BlobMetadata;

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

    pub async fn create_tables(self: &Self) -> Result<()> {
        sqlx::query!(
            r#"CREATE TABLE IF NOT EXISTS blobs (
                passphrase_hash BYTEA NOT NULL PRIMARY KEY,
                filename VARCHAR(255) NOT NULL,
                content_nonce BYTEA NOT NULL,
                filename_nonce BYTEA NOT NULL
            )"#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_blob(self: &Self, metadata: &BlobMetadata) -> Result<()> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_blob(self: &Self, passphrase_hash: &[u8]) -> Result<BlobMetadata> {
        let metadata = sqlx::query_as!(
            BlobMetadata,
            r#"SELECT * FROM blobs
            WHERE passphrase_hash = $1"#,
            passphrase_hash
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(metadata)
    }
}
