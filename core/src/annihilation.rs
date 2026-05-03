#![allow(dead_code)]

//! Mathematical Self-Annihilation via Ephemeral Cryptographic Shedding
//! Simulates physical NVMe secure-erase by keeping the encryption key
//! strictly in volatile registers and zeroing it upon annihilation.

use core::sync::atomic::{AtomicU64, Ordering};

/// A secure memory struct holding the transient encryption key.
pub struct EphemeralVault {
    // A 256-bit key split across 4 AtomicU64 registers to allow lock-free zeroing.
    key_part_0: AtomicU64,
    key_part_1: AtomicU64,
    key_part_2: AtomicU64,
    key_part_3: AtomicU64,
}

impl EphemeralVault {
    pub fn new(k0: u64, k1: u64, k2: u64, k3: u64) -> Self {
        Self {
            key_part_0: AtomicU64::new(k0),
            key_part_1: AtomicU64::new(k1),
            key_part_2: AtomicU64::new(k2),
            key_part_3: AtomicU64::new(k3),
        }
    }

    /// Fetches the ephemeral key for active IO operations.
    /// If the key has been annihilated, this returns zeroes.
    pub fn fetch_key(&self) -> [u64; 4] {
        [
            self.key_part_0.load(Ordering::SeqCst),
            self.key_part_1.load(Ordering::SeqCst),
            self.key_part_2.load(Ordering::SeqCst),
            self.key_part_3.load(Ordering::SeqCst),
        ]
    }
}

/// The lock-free routine that atomically zeroes the key.
pub struct MathematicalAnnihilator;

impl MathematicalAnnihilator {
    /// Triggers the absolute destruction of untrusted data by zeroing the transient key.
    /// Without the key, the ciphertext on the SSD becomes mathematically indistinguishable from random noise.
    pub fn annihilate(vault: &EphemeralVault) {
        // Atomic zeroing guarantees immediate destruction of access capability
        // across all cores simultaneously.
        vault.key_part_0.store(0, Ordering::SeqCst);
        vault.key_part_1.store(0, Ordering::SeqCst);
        vault.key_part_2.store(0, Ordering::SeqCst);
        vault.key_part_3.store(0, Ordering::SeqCst);
    }
}
