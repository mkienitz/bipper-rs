use crate::models::BlobMetadata;
use anyhow::{anyhow, Result};
use rand::rngs::OsRng;
use rand::Rng;
use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM, NONCE_LEN,
};

use ring::digest::{digest, SHA256};

use std::{format, fs, io::Read, path::Path, str::FromStr};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

fn entropy_to_key(entropy: &[u8; 32]) -> Result<UnboundKey> {
    UnboundKey::new(&AES_256_GCM, entropy).map_err(|e| anyhow!(e))
}

fn mnemonic_to_key(mnemonic: &str) -> Result<UnboundKey> {
    let entropy = bip39::Mnemonic::from_str(mnemonic)?.to_entropy();
    entropy_to_key(entropy.as_slice().try_into()?)
}

pub fn mnemonic_to_hash(mnemonic: &str) -> Result<String> {
    let entropy = bip39::Mnemonic::from_str(mnemonic)?.to_entropy();
    Ok(hex::encode(digest(&SHA256, &entropy)))
}

pub fn restore_filename(
    mnemonic: &str,
    mut filename_cipher: Vec<u8>,
    filename_nonce: [u8; NONCE_LEN],
) -> Result<String> {
    let mut filename_key = OpeningKey::new(
        mnemonic_to_key(mnemonic)?,
        SimpleNonceSequence(filename_nonce),
    );
    match filename_key.open_in_place(Aad::empty(), filename_cipher.as_mut()) {
        Ok(plaintext) => Ok(String::from_utf8(plaintext.to_vec())?),
        Err(e) => Err(anyhow!("Failed to decrypt filename: {}", e)),
    }
}

fn get_random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill(&mut nonce_bytes);
    nonce_bytes
}

struct SimpleNonceSequence([u8; NONCE_LEN]);

impl NonceSequence for SimpleNonceSequence {
    fn advance(&mut self) -> std::result::Result<Nonce, ring::error::Unspecified> {
        Ok(Nonce::assume_unique_for_key(self.0))
    }
}

pub struct Encryptor {
    file: File,
    // mnemonic: bip39::Mnemonic,
    entropy: [u8; 32],
    metadata: BlobMetadata,
}

impl Encryptor {
    pub async fn new(filename: &str) -> Result<Self> {
        // Create entropy and nonces
        let mut entropy = [0u8; 32];
        OsRng.fill(&mut entropy);
        let filename_nonce_bytes = get_random_nonce();

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
            // mnemonic: bip39::Mnemonic::from_entropy(&entropy)?,
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
        let nonce = get_random_nonce();
        let mut key = SealingKey::new(entropy_to_key(&self.entropy)?, SimpleNonceSequence(nonce));
        // NOTE: assuming appending is faster than two IO calls
        key.seal_in_place_append_tag(Aad::empty(), &mut chunk_vec)
            .map_err(anyhow::Error::msg)?;
        let mut finished_chunk = (chunk_vec.len() as u64).to_be_bytes().to_vec();
        finished_chunk.append(nonce.to_vec().as_mut());
        finished_chunk.append(chunk_vec.as_mut());
        self.file.write_all(finished_chunk.as_ref()).await?;
        Ok(())
    }

    pub async fn finalize(self) -> Result<(String, BlobMetadata)> {
        let mnemonic = bip39::Mnemonic::from_entropy(&self.entropy)?.to_string();
        Ok((mnemonic, self.metadata))
    }
}

pub struct DecryptionIter {
    file: fs::File,
    entropy: [u8; 32],
}

impl DecryptionIter {
    pub fn new(filename: &str, mnemonic: &str) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(filename)?;
        let entropy = bip39::Mnemonic::from_str(mnemonic)?.to_entropy()[..32].try_into()?;
        Ok(Self { file, entropy })
    }
}

impl Iterator for DecryptionIter {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to read length field
        let mut length_bytes = [0u8; 8];
        if self.file.read_exact(&mut length_bytes).is_err() {
            // Signal that we are done
            return None;
        }

        // Try to extract nonce bytes
        let mut nonce_bytes = [0u8; NONCE_LEN];
        if let Err(e) = self.file.read_exact(&mut nonce_bytes) {
            return Some(Err(anyhow!("Could not read nonce bytes: {}", e)));
        }

        // Try to read ciphertext
        let length = match u64::from_be_bytes(length_bytes).try_into() {
            Ok(l) => l,
            Err(e) => return Some(Err(anyhow!("Could not decode length: {}", e))),
        };

        let mut chunk = vec![0; length];
        match self.file.read_exact(&mut chunk) {
            Ok(()) => {
                let nonce = SimpleNonceSequence(nonce_bytes);
                let unbound_key = entropy_to_key(&self.entropy).ok()?;
                let mut key = OpeningKey::new(unbound_key, nonce);
                let plain = key.open_in_place(Aad::empty(), &mut chunk).ok()?;
                Some(Ok(plain.to_vec()))
            }
            Err(e) => Some(Err(e.into())),
        }
    }
}
