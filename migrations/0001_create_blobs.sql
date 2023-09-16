CREATE TABLE IF NOT EXISTS blobs (
	passphrase_hash BYTEA NOT NULL PRIMARY KEY,
	filename BYTEA NOT NULL,
	content_nonce BYTEA NOT NULL,
	filename_nonce BYTEA NOT NULL,
	cipher_hash VARCHAR(255) NOT NULL
)
