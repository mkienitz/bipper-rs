CREATE TABLE IF NOT EXISTS blobs (
	entropy_hash VARCHAR(255) NOT NULL PRIMARY KEY,
	filename_cipher BYTEA NOT NULL,
	filename_nonce BYTEA NOT NULL
)
