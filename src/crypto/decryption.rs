use anyhow::{anyhow, Result};
use bip39::Mnemonic;
use ring::aead::{Aad, BoundKey, OpeningKey, NONCE_LEN};
use std::{fs, io::Read, str::FromStr};

use super::{entropy_to_key, SimpleNonceSequence};

pub struct DecryptionIter {
    file: fs::File,
    entropy: [u8; 32],
}

pub fn restore_filename(
    mnemonic: &str,
    mut filename_cipher: Vec<u8>,
    filename_nonce: [u8; NONCE_LEN],
) -> Result<String> {
    let entropy = Mnemonic::from_str(mnemonic)?.to_entropy();
    let mut filename_key = OpeningKey::new(
        entropy_to_key(entropy.as_slice().try_into()?)?,
        SimpleNonceSequence(filename_nonce),
    );
    match filename_key.open_in_place(Aad::empty(), filename_cipher.as_mut()) {
        Ok(plaintext) => Ok(String::from_utf8(plaintext.to_vec())?),
        Err(e) => Err(anyhow!("Failed to decrypt filename: {}", e)),
    }
}

impl DecryptionIter {
    pub fn new(filename: &str, mnemonic: &str) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(filename)?;
        let entropy = Mnemonic::from_str(mnemonic)?
            .to_entropy()
            .as_slice()
            .try_into()?;
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
