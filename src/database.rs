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
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS blobs (
                entropy_hash VARCHAR(255) NOT NULL PRIMARY KEY,
                filename_cipher BYTEA NOT NULL,
                filename_nonce BYTEA NOT NULL
            )"#
        ).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn insert_blob(&self, metadata: &BlobMetadata) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO blobs
                    (entropy_hash, filename_cipher, filename_nonce)
                    VALUES ($1, $2, $3)"#,
        )
        .bind(&metadata.entropy_hash)
        .bind(&metadata.filename_cipher)
        .bind(metadata.filename_nonce)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_blob(&self, entropy_hash: &String) -> Result<BlobMetadata> {
        let metadata =
            sqlx::query_as::<_, BlobMetadata>(r#"SELECT * FROM blobs WHERE entropy_hash = $1"#)
                .bind(entropy_hash)
                .fetch_one(&self.pool)
                .await?;
        Ok(metadata)
    }
    pub async fn delete_blob(&self, entropy_hash: &String) -> Result<BlobMetadata> {
        let metadata = sqlx::query_as::<_, BlobMetadata>(
            r#"DELETE FROM blobs WHERE entropy_hash = $1 RETURNING *"#,
        )
        .bind(entropy_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(metadata)
    }
}
