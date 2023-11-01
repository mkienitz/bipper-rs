use std::str::FromStr;

use anyhow::{anyhow, Result};
use bip39::Mnemonic;
use ring::{
    aead::{Nonce, NonceSequence, UnboundKey, AES_256_GCM, NONCE_LEN},
    digest::{digest, SHA256},
};

pub struct SimpleNonceSequence(pub [u8; NONCE_LEN]);

impl NonceSequence for SimpleNonceSequence {
    fn advance(&mut self) -> std::result::Result<Nonce, ring::error::Unspecified> {
        Ok(Nonce::assume_unique_for_key(self.0))
    }
}

pub fn mnemonic_to_hash(mnemonic: &str) -> Result<String> {
    let entropy = Mnemonic::from_str(mnemonic)?.to_entropy();
    Ok(hex::encode(digest(&SHA256, &entropy)))
}

pub fn entropy_to_key(entropy: &[u8; 32]) -> Result<UnboundKey> {
    UnboundKey::new(&AES_256_GCM, entropy).map_err(|e| anyhow!(e))
}
