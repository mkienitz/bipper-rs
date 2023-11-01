use crate::models::BlobMetadata;
use anyhow::Result;
use bip39::Mnemonic;
use rand::{rngs::OsRng, Rng};
use ring::{
    aead::{Aad, BoundKey, SealingKey, NONCE_LEN},
    digest::{digest, SHA256},
};

use std::{format, path::Path};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

use super::{entropy_to_key, SimpleNonceSequence};

pub struct Encryptor {
    file: File,
    entropy: [u8; 32],
    metadata: BlobMetadata,
}

impl Encryptor {
    fn get_random_nonce() -> [u8; NONCE_LEN] {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill(&mut nonce_bytes);
        nonce_bytes
    }

    pub async fn new(filename: &str) -> Result<Self> {
        // Create entropy and nonces
        let mut entropy = [0u8; 32];
        OsRng.fill(&mut entropy);
        let filename_nonce_bytes = Self::get_random_nonce();

        // Digest passphrase entropy to be used as DB key and filename
        // let entropy_hash = digest(&SHA256, &entropy).as_ref().try_into()?;
        let entropy_hash = hex::encode(digest(&SHA256, &entropy));

        // Encrypt filename
        let filename_nonce_seq = SimpleNonceSequence(filename_nonce_bytes);
        let mut filename_key = SealingKey::new(entropy_to_key(&entropy)?, filename_nonce_seq);
        let mut filename_bytes = filename.to_string().into_bytes();
        filename_key
            .seal_in_place_append_tag(Aad::empty(), &mut filename_bytes)
            .map_err(anyhow::Error::msg)?;

        // Open file
        let path_str = format!("store/{}", entropy_hash);
        let file_path = Path::new(&path_str);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_path)
            .await?;

        Ok(Encryptor {
            file,
            entropy,
            metadata: BlobMetadata {
                entropy_hash,
                filename_cipher: filename_bytes,
                filename_nonce: filename_nonce_bytes,
            },
        })
    }

    pub async fn update(&mut self, chunk: &[u8]) -> Result<()> {
        let mut chunk_vec = chunk.to_vec();
        let nonce = Self::get_random_nonce();
        let mut key = SealingKey::new(entropy_to_key(&self.entropy)?, SimpleNonceSequence(nonce));
        key.seal_in_place_append_tag(Aad::empty(), &mut chunk_vec)
            .map_err(anyhow::Error::msg)?;
        let mut finished_chunk = (chunk_vec.len() as u64).to_be_bytes().to_vec();
        finished_chunk.append(nonce.to_vec().as_mut());
        finished_chunk.append(chunk_vec.as_mut());
        self.file.write_all(finished_chunk.as_ref()).await?;
        Ok(())
    }

    pub async fn finalize(self) -> Result<(String, BlobMetadata)> {
        let mnemonic = Mnemonic::from_entropy(&self.entropy)?.to_string();
        Ok((mnemonic, self.metadata))
    }
}
