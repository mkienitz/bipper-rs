use std::str::FromStr;

use aes_gcm::{aead::OsRng, AeadCore, AeadInPlace, Aes256Gcm, KeyInit};
use anyhow::{anyhow, Result};
use rand::Rng;
use scrypt::scrypt;

use crate::models::BlobMetadata;

type KeyType = aes_gcm::Key<aes_gcm::Aes256Gcm>;
type NonceType = aes_gcm::Nonce<aes_gcm::aes::cipher::typenum::U12>;

fn derive_key(entropy: &[u8], tag: &str) -> Result<KeyType> {
    let mut tagged_entropy = entropy.to_vec();
    tagged_entropy.append(&mut tag.as_bytes().to_vec());
    let mut key_buffer = [0u8; 32];
    let params = scrypt::Params::new(15, 8, 1, 32).map_err(|e| anyhow!(e))?;
    // This is perfectly fine
    let pepper = [0x46u8, 0xee, 0x5f, 0x18, 0x2c, 0xb8, 0x6d, 0x60];
    scrypt(&tagged_entropy, &pepper, &params, &mut key_buffer).map_err(|e| anyhow!(e))?;
    Ok(KeyType::clone_from_slice(&key_buffer))
}

pub fn encrypt(bytes: &mut Vec<u8>, key: KeyType) -> Result<NonceType> {
    // Accomodate authenticated data
    bytes.reserve_exact(12);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    cipher
        .encrypt_in_place(&nonce, &[], bytes)
        .map_err(|e| anyhow!(e))?;
    Ok(nonce)
}

pub fn decrypt(bytes: &mut Vec<u8>, key: &KeyType, nonce: &NonceType) -> Result<()> {
    let cipher = Aes256Gcm::new(&key);
    cipher
        .decrypt_in_place(&nonce, &[], bytes)
        .map_err(|e| anyhow!(e))?;
    Ok(())
}

pub async fn encrypt_file(bytes: &mut Vec<u8>, filename: &str) -> Result<(String, BlobMetadata)> {
    // Generate entropy and human-readable representation
    let mut entropy = [0u8; 32];
    rand::thread_rng().fill(&mut entropy);
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy)?.to_string();
    // Encrypt and produce metadata
    let passphrase_hash = derive_key(&entropy, "passphrase")?;
    let content_key = derive_key(&entropy, "content")?;
    let filename_key = derive_key(&entropy, "filename")?;
    let content_nonce = encrypt(bytes, content_key)?;
    let mut filename_bytes: Vec<u8> = filename.into();
    let filename_nonce = encrypt(&mut filename_bytes, filename_key)?;
    Ok((
        mnemonic,
        BlobMetadata {
            passphrase_hash: passphrase_hash.to_vec(),
            filename: filename_bytes,
            content_nonce: content_nonce.to_vec(),
            filename_nonce: filename_nonce.to_vec(),
        },
    ))
}

pub async fn calculate_passphrase_hash(mnemonic: &str) -> Result<Vec<u8>> {
    let entropy = bip39::Mnemonic::from_str(&mnemonic)?.to_entropy();
    Ok(derive_key(&entropy, "passphrase")?.to_vec())
}

pub async fn decrypt_file(
    content_bytes: &mut Vec<u8>,
    mnemonic: &str,
    meta: &mut BlobMetadata,
) -> Result<()> {
    let entropy = bip39::Mnemonic::from_str(&mnemonic)?.to_entropy();

    let content_key = derive_key(&entropy, "content")?;
    let filename_key = derive_key(&entropy, "filename")?;

    decrypt(
        &mut meta.filename,
        &filename_key,
        NonceType::from_slice(&meta.filename_nonce),
    )?;
    decrypt(
        content_bytes,
        &content_key,
        NonceType::from_slice(&meta.content_nonce),
    )?;
    Ok(())
}
