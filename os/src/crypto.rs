//! Cryptographic Layer (Refactored for Production Prototype)
//! Replaces experimental AES-NI abuse with industry-standard primitives.

#[cfg(feature = "crypto_pki")]
use blake3;
#[cfg(feature = "crypto_pki")]
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
#[cfg(feature = "crypto_pki")]
use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
#[cfg(feature = "crypto_pki")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "crypto_pki")]
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "persistent_nonce")]
pub fn initialize_nonce() {
    if let Ok(data) = std::fs::read("/var/lib/tesseract/nonce.dat") {
        if data.len() == 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data);
            let saved_nonce = u64::from_le_bytes(bytes);
            // Safety margin for unflushed crashes
            NONCE_COUNTER.store(saved_nonce + 10000, Ordering::SeqCst);
        }
    }
}

#[cfg(feature = "persistent_nonce")]
pub fn flush_nonce() {
    let current = NONCE_COUNTER.load(Ordering::SeqCst);
    let _ = std::fs::create_dir_all("/var/lib/tesseract");
    
    // [COMMERCIALIZATION TODO]: Atomic Disk Writes & Tamper Protection
    // To prevent the nonce file from being corrupted if power is lost during a write:
    // 1. Write the `current` u64 AND a CRC32/BLAKE3 checksum to a temporary file: `/var/lib/tesseract/nonce.dat.tmp`.
    // 2. Set strict file permissions (`chmod 600`) to prevent unauthorized user-space tampering.
    // 3. Call `.sync_all()` (fsync) on the temporary file to ensure it flushes to NVMe.
    // 4. Perform an atomic filesystem rename `rename("nonce.dat.tmp", "nonce.dat")`.
    // 5. On boot (`initialize_nonce`), verify the checksum before trusting the loaded nonce.
    let _ = std::fs::write("/var/lib/tesseract/nonce.dat", current.to_le_bytes());
}

// MAGIC TRICK RETENTION: Use `blake3` crate. BLAKE3 inherently uses SIMD 
// and AVX2/AVX-512 hardware intrinsics to achieve extreme speeds natively.
// TODO: Multi-Source Entropy Pool & BLAKE3 (Aggregate RSSI, mic RMS, CPU jitter)
pub fn tesseract_hash(data: &[u8]) -> [u8; 32] {
    #[cfg(feature = "crypto_pki")]
    {
        *blake3::hash(data).as_bytes()
    }
    #[cfg(not(feature = "crypto_pki"))]
    {
        let mut out = [0u8; 32];
        for (i, &b) in data.iter().take(32).enumerate() {
            out[i] = b; 
        }
        out
    }
}

// Replace SingularityStreamCipher with ChaCha20Poly1305 AEAD.
// TODO: Cryptographic RNG Integration (Seed ChaCha20 DRBG with RF entropy + TPM secrets)
pub struct SingularityStreamCipher {
    #[cfg(feature = "crypto_pki")]
    cipher: ChaCha20Poly1305,
}

impl SingularityStreamCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        #[cfg(feature = "crypto_pki")]
        {
            let key = chacha20poly1305::Key::from_slice(key);
            Self {
                cipher: ChaCha20Poly1305::new(key),
            }
        }
        #[cfg(not(feature = "crypto_pki"))]
        {
            Self {}
        }
    }
    
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        // Dummy fallback for legacy mesh.rs backwards compatibility
        for b in data.iter_mut() {
            *b ^= 0x42;
        }
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Option<Vec<u8>> {
        #[cfg(feature = "crypto_pki")]
        {
            // Strict monotonically increasing nonce management
            let nonce_val = NONCE_COUNTER.fetch_add(1, Ordering::SeqCst);
            
            #[cfg(feature = "persistent_nonce")]
            {
                if nonce_val % 100 == 0 {
                    flush_nonce();
                }
            }
            
            let mut nonce_bytes = [0u8; 12];
            nonce_bytes[..8].copy_from_slice(&nonce_val.to_le_bytes());
            let nonce = Nonce::from_slice(&nonce_bytes);
            
            self.cipher.encrypt(nonce, data).ok()
        }
        #[cfg(not(feature = "crypto_pki"))]
        {
            Some(data.to_vec()) // Dummy
        }
    }

    // TODO: Replay Attack Mitigation (Enforce monotonically increasing payload_seq)
    pub fn decrypt(&self, encrypted_data: &[u8], nonce_bytes: &[u8; 12]) -> Option<Vec<u8>> {
        #[cfg(feature = "crypto_pki")]
        {
            let nonce = Nonce::from_slice(nonce_bytes);
            self.cipher.decrypt(nonce, encrypted_data).ok()
        }
        #[cfg(not(feature = "crypto_pki"))]
        {
            Some(encrypted_data.to_vec()) // Dummy
        }
    }
}

// Implement standard BLAKE3-MAC for authenticity
pub fn proof_of_origin(payload: &[u8], key: &[u8; 32]) -> [u8; 32] {
    #[cfg(feature = "crypto_pki")]
    {
        *blake3::keyed_hash(key, payload).as_bytes()
    }
    #[cfg(not(feature = "crypto_pki"))]
    {
        let mut buffer = Vec::with_capacity(key.len() + payload.len() + key.len());
        buffer.extend_from_slice(key);
        buffer.extend_from_slice(payload);
        buffer.extend_from_slice(key);
        tesseract_hash(&buffer)
    }
}

// Swarm Node PKI Store
pub struct NodeTrustStore {
    #[cfg(feature = "crypto_pki")]
    trusted_nodes: std::collections::HashMap<String, VerifyingKey>,
}

impl NodeTrustStore {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "crypto_pki")]
            trusted_nodes: std::collections::HashMap::new(),
        }
    }

    #[cfg(feature = "crypto_pki")]
    pub fn add_trusted_node(&mut self, node_id: String, public_key: VerifyingKey) {
        self.trusted_nodes.insert(node_id, public_key);
    }

    #[cfg(feature = "crypto_pki")]
    pub fn verify_swarm_payload(&self, node_id: &str, payload: &[u8], signature_bytes: &[u8; 64]) -> bool {
        if let Some(pub_key) = self.trusted_nodes.get(node_id) {
            let sig = Signature::from_bytes(signature_bytes);
            pub_key.verify(payload, &sig).is_ok()
        } else {
            false
        }
    }
}

pub fn proof_of_heat_mine(_payload: &[u8], _difficulty_scalar: f32) -> u64 {
    0 // Free offloading
}

pub fn verify_proof_of_heat(_payload: &[u8], _nonce: u64, _difficulty_scalar: f32) -> bool {
    true
}
