use crate::models::BlobMetadata;
use aes::cipher::{
    block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, BlockSizeUser, IvSizeUser, KeyIvInit,
    KeySizeUser,
};
use anyhow::{anyhow, Result};
use generic_array::typenum::Unsigned;
use rand::rngs::OsRng;
use rand::Rng;
use scrypt::scrypt;
use std::{format, fs, io::Read, path::Path, str::FromStr};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

pub fn derive_key(entropy: &[u8], tag: &str) -> Result<Vec<u8>> {
    let mut tagged_entropy = entropy.to_vec();
    tagged_entropy.append(&mut tag.as_bytes().to_vec());
    let mut key_buffer = [0u8; KEY_SIZE];
    let params = scrypt::Params::new(15, 8, 1, KEY_SIZE).map_err(|e| anyhow!(e))?;
    // This is perfectly fine
    let pepper = [0x46u8, 0xee, 0x5f, 0x18, 0x2c, 0xb8, 0x6d, 0x60];
    scrypt(&tagged_entropy, &pepper, &params, &mut key_buffer).map_err(|e| anyhow!(e))?;
    Ok(key_buffer.to_vec())
}

pub async fn calculate_passphrase_hash(mnemonic: &str) -> Result<Vec<u8>> {
    let entropy = bip39::Mnemonic::from_str(mnemonic)?.to_entropy();
    derive_key(&entropy, "passphrase")
}

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
const BLOCK_SIZE: usize = <Aes256CbcEnc as BlockSizeUser>::BlockSize::USIZE;
const NONCE_SIZE: usize = <Aes256CbcEnc as IvSizeUser>::IvSize::USIZE;
const KEY_SIZE: usize = <Aes256CbcEnc as KeySizeUser>::KeySize::USIZE;

pub struct EncryptionState {
    encryptor: Aes256CbcEnc,
    file: File,
    file_path: Box<Path>,
    buffer: Vec<u8>,
    mnemonic: bip39::Mnemonic,
    metadata: BlobMetadata,
}

impl EncryptionState {
    pub async fn new(filename: &str) -> Result<Self> {
        // Create entropy and nonces
        let mut entropy = [0u8; 32];
        let mut filename_nonce = [0u8; NONCE_SIZE];
        let mut content_nonce = [0u8; NONCE_SIZE];
        OsRng.fill(&mut entropy);
        OsRng.fill(&mut filename_nonce);
        OsRng.fill(&mut content_nonce);

        // Derive keys and create nonces
        let passphrase_hash = derive_key(&entropy, "passphrase")?;
        let content_key = derive_key(&entropy, "content")?;
        let filename_key = derive_key(&entropy, "filename")?;

        // Setup cipher and encrypt filename
        let filename_encryptor = Aes256CbcEnc::new(
            filename_key.as_slice().into(),
            filename_nonce.as_ref().into(),
        );
        let filename_bytes =
            filename_encryptor.encrypt_padded_vec_mut::<Pkcs7>(filename.as_bytes());

        // Setup encryptor
        let encryptor =
            Aes256CbcEnc::new(content_key.as_slice().into(), content_nonce.as_ref().into());

        // Open file
        let str_path = format!("store/{}", hex::encode(passphrase_hash.clone()));
        let file_path = Path::new(&str_path);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_path)
            .await?;

        Ok(EncryptionState {
            encryptor,
            file,
            file_path: file_path.into(),
            buffer: Vec::new(),
            mnemonic: bip39::Mnemonic::from_entropy(&entropy)?,
            metadata: BlobMetadata {
                passphrase_hash: passphrase_hash.to_vec(),
                filename: filename_bytes,
                content_nonce: content_nonce.to_vec(),
                filename_nonce: filename_nonce.to_vec(),
                cipher_hash: "".to_string(),
            },
        })
    }

    pub async fn update(&mut self, chunk: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(chunk);
        let chunks = self.buffer.chunks_exact_mut(Aes256CbcEnc::block_size());
        for chunk in chunks {
            self.encryptor.encrypt_block_mut(chunk.into());
        }
        let superblock_size = self.buffer.len() - self.buffer.len() % BLOCK_SIZE;
        let drained = self.buffer.drain(..superblock_size);
        self.file.write_all(drained.as_ref()).await?;
        Ok(())
    }

    pub async fn finalize(mut self) -> Result<(String, BlobMetadata)> {
        let mut rem_buf = [0u8; BLOCK_SIZE];
        if !self.buffer.is_empty() {
            self.encryptor
                .encrypt_padded_b2b_mut::<Pkcs7>(&self.buffer, &mut rem_buf)
                .map_err(|e| anyhow!(e))?;
            self.file.write_all(&rem_buf).await?;
        }
        self.metadata.cipher_hash = sha256::try_digest(self.file_path)?;
        Ok((self.mnemonic.to_string(), self.metadata))
    }
}

const DECRYPTION_BUFSIZE: usize = 32;
pub struct DecryptionState {
    file: fs::File,
    decryptor: Aes256CbcDec,
    entropy: Vec<u8>,
    metadata: BlobMetadata,
    buffer: Box<[u8; DECRYPTION_BUFSIZE]>,
    buf_used: usize,
    remaining_length: u64,
}

impl Iterator for DecryptionState {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.file.read(&mut self.buffer[self.buf_used..]) {
            Ok(0) => None,
            Ok(bytes_read) => {
                // New buffer size includes last call's remaining bytes
                let curr_size = bytes_read + self.buf_used;
                // Deal with unpadded blocks first
                // Make sure we don't greedily consume the potentially padded block
                //                                   ___
                let chunks = self.buffer[..curr_size - 1].chunks_exact_mut(BLOCK_SIZE);
                let mut n_decrypted = BLOCK_SIZE * chunks.len();
                for chunk in chunks {
                    self.decryptor.decrypt_block_mut(chunk.into());
                }
                self.remaining_length -= n_decrypted as u64;

                // Handle padded block
                if self.remaining_length == BLOCK_SIZE as u64 {
                    let padded_chunk = &mut self.buffer[n_decrypted..curr_size];
                    // The following clone is necessary because the Decryptor object consumes
                    // itself upon decrypt_padded(_mut)
                    let dc = self.decryptor.clone();
                    let unpadded_chunk = match dc.decrypt_padded_mut::<Pkcs7>(padded_chunk) {
                        Ok(slice) => slice,
                        Err(e) => return Some(Err(anyhow!(e).context("Failed to unpad"))),
                    };
                    n_decrypted += unpadded_chunk.len();
                    Some(Ok(self.buffer[..n_decrypted].to_vec()))
                } else {
                    let res = self.buffer[..n_decrypted].to_vec();
                    // Copy unprocessed bytes to the beginning of the buffer for the next iteration
                    self.buffer.copy_within(n_decrypted..curr_size, 0);
                    Some(Ok(res))
                }
            }
            Err(e) => {
                panic!("{e}")
            }
        }
    }
}

impl DecryptionState {
    pub async fn new(
        storage_path: String,
        mnemonic_str: &str,
        metadata: BlobMetadata,
    ) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(storage_path)?;

        let entropy = bip39::Mnemonic::from_str(mnemonic_str)?.to_entropy();
        let content_key = derive_key(&entropy, "content")?;

        let decryptor = Aes256CbcDec::new(
            content_key.as_slice().into(),
            metadata.content_nonce.as_slice().into(),
        );

        let remaining_length = file.metadata()?.len();

        Ok(DecryptionState {
            file,
            decryptor,
            entropy,
            metadata,
            buffer: Box::new([0u8; DECRYPTION_BUFSIZE]),
            buf_used: 0,
            remaining_length,
        })
    }

    pub fn filename(&self) -> Result<String> {
        let filename_key = derive_key(&self.entropy, "filename")?;
        let filename_decryptor = Aes256CbcDec::new(
            filename_key.as_slice().into(),
            self.metadata.filename_nonce.as_slice().into(),
        );

        Ok(String::from_utf8(
            filename_decryptor
                .decrypt_padded_vec_mut::<Pkcs7>(&self.metadata.filename)
                .map_err(|e| anyhow!(e))?,
        )?)
    }
}
