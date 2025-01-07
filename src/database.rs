use anyhow::Result;

use tokio_rusqlite::Connection;

use crate::models::BlobMetadata;

#[derive(Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(database_path: &str) -> Result<Self> {
        let conn = Connection::open(database_path).await?;
        conn.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS blobs (
                entropy_hash TEXT NOT NULL PRIMARY KEY,
                filename_cipher BLOB NOT NULL,
                filename_nonce BLOB NOT NULL
            )",
                (),
            )?;
            Ok(())
        })
        .await?;
        Ok(Database { conn })
    }

    pub async fn insert_blob(&self, metadata: &BlobMetadata) -> Result<()> {
        let meta: BlobMetadata = metadata.clone();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO blobs
                    (entropy_hash, filename_cipher, filename_nonce)
                    VALUES (?1, ?2, ?3)",
                    (meta.entropy_hash, meta.filename_cipher, meta.filename_nonce),
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn find_blob(&self, entropy_hash: &str) -> Result<BlobMetadata> {
        let entropy_hash = entropy_hash.to_owned();
        Ok(self
            .conn
            .call(move |conn| {
                Ok(conn.query_row(
                    "SELECT * FROM blobs WHERE entropy_hash = ?1",
                    [entropy_hash],
                    |row| {
                        Ok(BlobMetadata {
                            entropy_hash: row.get(0)?,
                            filename_cipher: row.get(1)?,
                            filename_nonce: row.get(2)?,
                        })
                    },
                )?)
            })
            .await?)
    }

    pub async fn delete_blob(&self, entropy_hash: &str) -> Result<BlobMetadata> {
        let blob = self.find_blob(entropy_hash).await?;
        let entropy_hash = entropy_hash.to_owned();
        Ok(self
            .conn
            .call(move |conn| {
                conn.execute("DELETE FROM blobs WHERE entropy_hash = ?1", [entropy_hash])?;
                Ok(blob)
            })
            .await?)
    }
}
