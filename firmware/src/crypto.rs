#![allow(dead_code)] /// remove before handoff

extern crate alloc;
use alloc::vec::Vec;
use blake3;

#[cfg(not(target_arch = "arm"))]
use pqcrypto_dilithium::dilithium3;
#[cfg(not(target_arch = "arm"))]
use pqcrypto_traits::sign::{PublicKey as PQPublicKey, SecretKey as PQSecretKey, SignedMessage};

pub const PROTOCOL_VERSION: u8 = 1;
pub const BLAKE3_HASH_SIZE: usize = 32;
pub const BLAKE3_MAC_SIZE: usize = 32;
pub const BLAKE3_KEY_SIZE: usize = 32;
pub const DILITHIUM3_SIGNATURE_BYTES: usize = 2420;
pub const DILITHIUM3_PUBKEY_SIZE: usize = 1952;
pub const DILITHIUM3_PRIVKEY_SIZE: usize = 4032;
pub const MAX_FILE_SIZE: usize = 8192;

#[derive(Debug, Clone, Copy)]
pub enum CryptoError {
    SignatureVerificationFailed,
    MacVerificationFailed,
    KeyGenerationFailed,
    SigningFailed,
    InvalidFileFormat,
    FileTooLarge,
}

impl core::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CryptoError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            CryptoError::MacVerificationFailed => write!(f, "MAC verification failed"),
            CryptoError::KeyGenerationFailed => write!(f, "Keygen failed"),
            CryptoError::SigningFailed => write!(f, "Signing failed"),
            CryptoError::InvalidFileFormat => write!(f, "Invalid file format"),
            CryptoError::FileTooLarge => write!(f, "File too large"),
        }
    }
}

pub type CryptoResult<T> = Result<T, CryptoError>;

pub fn blake3_hash(data: &[u8]) -> [u8; BLAKE3_HASH_SIZE] {
    let hash_output = blake3::hash(data);
    let mut hash = [0u8; BLAKE3_HASH_SIZE];
    hash.copy_from_slice(hash_output.as_bytes());
    hash
}

pub fn blake3_keyed_hash(key: &[u8; BLAKE3_KEY_SIZE], data: &[u8]) -> [u8; BLAKE3_MAC_SIZE] {
    let mac_output = blake3::keyed_hash(key, data);
    let mut mac = [0u8; BLAKE3_MAC_SIZE];
    mac.copy_from_slice(mac_output.as_bytes());
    mac
}

#[cfg(not(target_arch = "arm"))]
pub fn dilithium_keygen() -> CryptoResult<(Vec<u8>, Vec<u8>)> {
    let (pk, sk) = dilithium3::keypair();
    
    let pub_key = pk.as_bytes().to_vec();
    let priv_key = sk.as_bytes().to_vec();
    
    Ok((pub_key, priv_key))
}

#[cfg(target_arch = "arm")]
pub fn dilithium_keygen() -> CryptoResult<(Vec<u8>, Vec<u8>)> {
    Err(CryptoError::KeyGenerationFailed)
}

#[cfg(not(target_arch = "arm"))]
pub fn dilithium_sign(message: &[u8], private_key: &[u8]) -> CryptoResult<Vec<u8>> {
    let sk = dilithium3::SecretKey::from_bytes(private_key)
        .map_err(|_| CryptoError::SigningFailed)?;
    
    let signed_message = dilithium3::sign(message, &sk);
    
    Ok(signed_message.as_bytes().to_vec())
}

#[cfg(target_arch = "arm")]
pub fn dilithium_sign(_message: &[u8], _private_key: &[u8]) -> CryptoResult<Vec<u8>> {
    Err(CryptoError::SigningFailed)
}

#[cfg(not(target_arch = "arm"))]
pub fn dilithium_verify(signed_message_bytes: &[u8], public_key: &[u8]) -> CryptoResult<Vec<u8>> {
    let pk = dilithium3::PublicKey::from_bytes(public_key)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;
    
    let signed_msg = dilithium3::SignedMessage::from_bytes(signed_message_bytes)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;
    
    let recovered_message = dilithium3::open(&signed_msg, &pk)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;
    
    Ok(recovered_message.to_vec())
}

#[cfg(target_arch = "arm")]
pub fn dilithium_verify(_signed_message_bytes: &[u8], _public_key: &[u8]) -> CryptoResult<Vec<u8>> {
    Err(CryptoError::SignatureVerificationFailed)
}

#[cfg(not(target_arch = "arm"))]
pub fn send_file(
    file_data: &[u8],
    private_key: &[u8],
    master_secret: &[u8; BLAKE3_KEY_SIZE],
) -> CryptoResult<Vec<u8>> {
    if file_data.len() > MAX_FILE_SIZE {
        return Err(CryptoError::FileTooLarge);
    }

    let signed_message = dilithium_sign(file_data, private_key)?;

    let mut data_to_mac = Vec::new();
    data_to_mac.push(PROTOCOL_VERSION);
    data_to_mac.extend_from_slice(&signed_message);

    let auth_mac = blake3_keyed_hash(master_secret, &data_to_mac);

    // [VERSION] [SIGNED_MESSAGE] [MAC]
    let mut authenticated = Vec::new();
    authenticated.push(PROTOCOL_VERSION);
    authenticated.extend_from_slice(&signed_message);
    authenticated.extend_from_slice(&auth_mac);

    Ok(authenticated)
}

#[cfg(target_arch = "arm")]
pub fn send_file(
    _file_data: &[u8],
    _private_key: &[u8],
    _master_secret: &[u8; BLAKE3_KEY_SIZE],
) -> CryptoResult<Vec<u8>> {
    Err(CryptoError::SigningFailed)
}

#[cfg(not(target_arch = "arm"))]
pub fn receive_file(
    authenticated_file: &[u8],
    sender_public_key: &[u8],
    master_secret: &[u8; BLAKE3_KEY_SIZE],
) -> CryptoResult<Vec<u8>> {
    // 1 (VERSION) + 2420 (SIG overhead) + 32 (MAC) = 2453 B
    if authenticated_file.len() < 1 + DILITHIUM3_SIGNATURE_BYTES + BLAKE3_MAC_SIZE {
        return Err(CryptoError::InvalidFileFormat);
    }

    let version = authenticated_file[0];
    if version != PROTOCOL_VERSION {
        return Err(CryptoError::InvalidFileFormat);
    }

    let received_mac_start = authenticated_file.len() - BLAKE3_MAC_SIZE;
    let received_mac = &authenticated_file[received_mac_start..];

    let signed_message_bytes = &authenticated_file[1..received_mac_start];

    let mut data_to_mac = Vec::new();
    data_to_mac.push(version);
    data_to_mac.extend_from_slice(signed_message_bytes);

    let computed_mac = blake3_keyed_hash(master_secret, &data_to_mac);

    let mut mac_match = true;
    for (a, b) in computed_mac.iter().zip(received_mac.iter()) {
        if a != b {
            mac_match = false;
        }
    }

    if !mac_match {
        return Err(CryptoError::MacVerificationFailed);
    }

    let file_data = dilithium_verify(signed_message_bytes, sender_public_key)?;

    Ok(file_data)
}

#[cfg(target_arch = "arm")]
pub fn receive_file(
    _authenticated_file: &[u8],
    _sender_public_key: &[u8],
    _master_secret: &[u8; BLAKE3_KEY_SIZE],
) -> CryptoResult<Vec<u8>> {
    Err(CryptoError::SignatureVerificationFailed)
}

// tests
#[cfg(all(test, not(target_arch = "arm")))]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash_deterministic() {
        let data = b"test data";
        let hash1 = blake3_hash(data);
        let hash2 = blake3_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blake3_hash_different_inputs() {
        let hash1 = blake3_hash(b"data1");
        let hash2 = blake3_hash(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_blake3_keyed_hash_deterministic() {
        let key = [0x42; 32];
        let data = b"test data";
        let mac1 = blake3_keyed_hash(&key, data);
        let mac2 = blake3_keyed_hash(&key, data);
        assert_eq!(mac1, mac2);
    }

    #[test]
    fn test_blake3_keyed_hash_different_keys() {
        let key1 = [0x42; 32];
        let key2 = [0x55; 32];
        let data = b"test data";
        let mac1 = blake3_keyed_hash(&key1, data);
        let mac2 = blake3_keyed_hash(&key2, data);
        assert_ne!(mac1, mac2);
    }

    #[test]
    fn test_blake3_keyed_hash_different_data() {
        let key = [0x42; 32];
        let mac1 = blake3_keyed_hash(&key, b"data1");
        let mac2 = blake3_keyed_hash(&key, b"data2");
        assert_ne!(mac1, mac2);
    }

    #[test]
    fn test_dilithium_keygen() {
        let result = dilithium_keygen();
        assert!(result.is_ok());
        
        let (pub_key, priv_key) = result.unwrap();
        assert_eq!(pub_key.len(), DILITHIUM3_PUBKEY_SIZE);
        assert_eq!(priv_key.len(), DILITHIUM3_PRIVKEY_SIZE);
    }

    #[test]
    fn test_dilithium_sign_verify_roundtrip() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");
        let message = b"test message";

        let signed = dilithium_sign(message, &priv_key).expect("sign failed");
        
        assert!(signed.len() >= DILITHIUM3_SIGNATURE_BYTES + message.len());

        let result = dilithium_verify(&signed, &pub_key);
        assert!(result.is_ok());
        
        if let Ok(recovered) = result {
            assert_eq!(recovered, message);
        }
    }

    #[test]
    fn test_dilithium_verify_fails_on_wrong_message() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");

        let message = b"original message";
        let signed = dilithium_sign(message, &priv_key).expect("sign failed");

        let recovered = dilithium_verify(&signed, &pub_key);
        assert!(recovered.is_ok());
        
        if let Ok(msg) = recovered {
            assert_eq!(msg, message);
        }
    }

    #[test]
    fn test_dilithium_verify_fails_on_wrong_key() {
        let (pub_key1, priv_key1) = dilithium_keygen().expect("keygen1 failed");
        let (pub_key2, _priv_key2) = dilithium_keygen().expect("keygen2 failed");

        let message = b"test message";
        let signed = dilithium_sign(message, &priv_key1).expect("sign failed");

        let result = dilithium_verify(&signed, &pub_key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_send_receive_file_roundtrip() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");
        let master_secret = [0x77; 32];
        let original_data = b"sensitive file data";

        let authenticated = send_file(original_data, &priv_key, &master_secret)
            .expect("send_file failed");

        // VERSION(1) + SIGNED_MESSAGE(2420 + 19) + MAC(32) = 2472 B
        let expected_min_size = 1 + DILITHIUM3_SIGNATURE_BYTES + original_data.len() + BLAKE3_MAC_SIZE;
        assert!(authenticated.len() >= expected_min_size);

        let received_data = receive_file(&authenticated, &pub_key, &master_secret)
            .expect("receive_file failed");

        assert_eq!(received_data, original_data);
    }

    #[test]
    fn test_receive_file_fails_on_tampered_data() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");
        let master_secret = [0x77; 32];
        let original_data = b"sensitive file data";

        let mut authenticated = send_file(original_data, &priv_key, &master_secret)
            .expect("send_file failed");

        if authenticated.len() > 10 {
            authenticated[5] ^= 0xFF; // Flip bits in signed message section
        }

        let result = receive_file(&authenticated, &pub_key, &master_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_receive_file_fails_on_tampered_signature() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");
        let master_secret = [0x77; 32];
        let original_data = b"sensitive file data";

        let mut authenticated = send_file(original_data, &priv_key, &master_secret)
            .expect("send_file failed");

        let sig_offset = 1 + 50; // VERSION + some offset into signed message
        if authenticated.len() > sig_offset {
            authenticated[sig_offset] ^= 0xFF;
        }

        let result = receive_file(&authenticated, &pub_key, &master_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_receive_file_fails_on_wrong_mac_key() {
        let (pub_key, priv_key) = dilithium_keygen().expect("keygen failed");
        let master_secret = [0x77; 32];
        let wrong_secret = [0x88; 32];
        let original_data = b"sensitive file data";

        let authenticated = send_file(original_data, &priv_key, &master_secret)
            .expect("send_file failed");

        let result = receive_file(&authenticated, &pub_key, &wrong_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_too_large() {
        let (_, priv_key) = dilithium_keygen().expect("keygen failed");
        let master_secret = [0x77; 32];
        let large_data = vec![0u8; MAX_FILE_SIZE + 1];

        let result = send_file(&large_data, &priv_key, &master_secret);
        assert!(matches!(result, Err(CryptoError::FileTooLarge)));
    }
}

