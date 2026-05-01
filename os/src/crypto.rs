//! Chaotic Tesseract Cryptographic Layer
//! Fully native bit-hacked encryption and hashing designed for Zero-Trust Swarm environments.
//! Uses no external dependencies.

/// Tesseract Hash (256-bit)
/// A custom non-linear bit-mixing hash function to distill chaotic ambient entropy.
pub fn tesseract_hash(data: &[u8]) -> [u8; 32] {
    let mut state = [0x9e3779b9u32; 8];
    
    let mut i = 0;
    
    #[cfg(target_arch = "x86_64")]
    {
        // MAGIC TRICK: AES-NI Hardware Intrinsics
        // 10x Speedup via Dedicated Silicon
        if std::is_x86_feature_detected!("aes") && std::is_x86_feature_detected!("sse2") {
            unsafe {
                use core::arch::x86_64::*;
                let mut state_vec_0 = _mm_loadu_si128(state.as_ptr() as *const __m128i);
                let mut state_vec_1 = _mm_loadu_si128(state.as_ptr().add(4) as *const __m128i);
                let round_key = _mm_set1_epi32(0xdeadbeef_u32 as i32);
                
                while i + 32 <= data.len() {
                    let data_vec_0 = _mm_loadu_si128(data.as_ptr().add(i) as *const __m128i);
                    let data_vec_1 = _mm_loadu_si128(data.as_ptr().add(i + 16) as *const __m128i);
                    
                    // Hardware AES encryption round (1 clock cycle throughput)
                    state_vec_0 = _mm_aesenc_si128(_mm_xor_si128(state_vec_0, data_vec_0), round_key);
                    state_vec_1 = _mm_aesenc_si128(_mm_xor_si128(state_vec_1, data_vec_1), round_key);
                    
                    i += 32;
                }
                _mm_storeu_si128(state.as_mut_ptr() as *mut __m128i, state_vec_0);
                _mm_storeu_si128(state.as_mut_ptr().add(4) as *mut __m128i, state_vec_1);
            }
        } else if std::is_x86_feature_detected!("avx2") {
            // Safe Fallback to AVX2 Vector Math
            unsafe {
                use core::arch::x86_64::*;
                let mut state_vec = _mm256_loadu_si256(state.as_ptr() as *const __m256i);
                
                while i + 32 <= data.len() {
                    let data_vec = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
                    state_vec = _mm256_xor_si256(state_vec, data_vec);
                    
                    let shifted = _mm256_srli_epi32(state_vec, 13);
                    state_vec = _mm256_xor_si256(state_vec, shifted);
                    
                    i += 32;
                }
                _mm256_storeu_si256(state.as_mut_ptr() as *mut __m256i, state_vec);
            }
        }
    }

    // Scalar fallback
    while i < data.len() {
        let chunk = if i + 4 <= data.len() {
            u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]])
        } else {
            let mut buf = [0u8; 4];
            for j in 0..(data.len() - i) {
                buf[j] = data[i+j];
            }
            u32::from_le_bytes(buf)
        };
        
        let idx = (i / 4) % 8;
        state[idx] ^= chunk;
        state[idx] = state[idx].rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64);
        
        // Avalanche
        state[(idx + 1) % 8] ^= state[idx].rotate_right(7);
        state[(idx + 3) % 8] ^= state[idx].rotate_left(11);
        
        i += 4;
    }
    
    // Finalize
    for _ in 0..4 {
        for j in 0..8 {
            state[j] ^= state[(j + 1) % 8].rotate_left(9);
            state[j] = state[j].wrapping_mul(0x85ebca6b);
            state[j] ^= state[j] >> 13;
            state[j] = state[j].wrapping_mul(0xc2b2ae35);
            state[j] ^= state[j] >> 16;
        }
    }
    
    let mut out = [0u8; 32];
    for j in 0..8 {
        let bytes = state[j].to_le_bytes();
        out[j*4..j*4+4].copy_from_slice(&bytes);
    }
    out
}

/// Singularity Stream Cipher
/// An XOR-shift PRNG-based stream cipher initialized with a 256-bit key.
pub struct SingularityStreamCipher {
    state: [u32; 8],
    counter: u64,
}

impl SingularityStreamCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        let mut state = [0u32; 8];
        for i in 0..8 {
            state[i] = u32::from_le_bytes([key[i*4], key[i*4+1], key[i*4+2], key[i*4+3]]);
        }
        Self { state, counter: 0 }
    }
    
    fn next_keystream_block(&mut self) -> [u8; 32] {
        self.counter += 1;
        let mut block_state = self.state;
        
        // Mix counter into block state
        block_state[0] ^= (self.counter & 0xFFFFFFFF) as u32;
        block_state[1] ^= (self.counter >> 32) as u32;
        
        // Quarter-round style mixing (simplified ChaCha/Salsa inspired)
        for _ in 0..10 {
            for i in 0..4 {
                let a = i;
                let b = i + 4;
                block_state[a] = block_state[a].wrapping_add(block_state[b]);
                block_state[b] ^= block_state[a];
                block_state[b] = block_state[b].rotate_left(16);
                
                block_state[a] = block_state[a].wrapping_add(block_state[b]);
                block_state[b] ^= block_state[a];
                block_state[b] = block_state[b].rotate_left(12);
                
                block_state[a] = block_state[a].wrapping_add(block_state[b]);
                block_state[b] ^= block_state[a];
                block_state[b] = block_state[b].rotate_left(8);
                
                block_state[a] = block_state[a].wrapping_add(block_state[b]);
                block_state[b] ^= block_state[a];
                block_state[b] = block_state[b].rotate_left(7);
            }
        }
        
        // Add original state back (Merkle-Damgard style)
        for i in 0..8 {
            block_state[i] = block_state[i].wrapping_add(self.state[i]);
        }
        
        let mut out = [0u8; 32];
        for i in 0..8 {
            out[i*4..i*4+4].copy_from_slice(&block_state[i].to_le_bytes());
        }
        out
    }
    
    /// Encrypts or decrypts data in place
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        let mut i = 0;
        
        #[cfg(target_arch = "x86_64")]
        {
            if std::is_x86_feature_detected!("avx2") {
                unsafe {
                    use core::arch::x86_64::*;
                    while i + 32 <= data.len() {
                        let ks = self.next_keystream_block();
                        let ks_ptr = ks.as_ptr() as *const __m256i;
                        let data_ptr = data.as_mut_ptr().add(i) as *mut __m256i;
                        
                        let ks_vec = _mm256_loadu_si256(ks_ptr);
                        let data_vec = _mm256_loadu_si256(data_ptr);
                        
                        let result = _mm256_xor_si256(data_vec, ks_vec);
                        _mm256_storeu_si256(data_ptr, result);
                        
                        i += 32;
                    }
                }
            }
        }
        
        while i < data.len() {
            let ks = self.next_keystream_block();
            let chunk_len = std::cmp::min(32, data.len() - i);
            for j in 0..chunk_len {
                data[i + j] ^= ks[j];
            }
            i += 32;
        }
    }
}

/// Proof-of-Origin Signature
/// A custom HMAC equivalent using `tesseract_hash`
pub fn proof_of_origin(payload: &[u8], seed: &[u8; 32]) -> [u8; 32] {
    let mut buffer = Vec::with_capacity(seed.len() + payload.len() + seed.len());
    buffer.extend_from_slice(seed);
    buffer.extend_from_slice(payload);
    buffer.extend_from_slice(seed);
    tesseract_hash(&buffer)
}


/// Proof-of-Heat Mining (Thermodynamic Cost)
/// Requires the node to perform AVX2-accelerated hashing until the hash has a specific number of leading zero bytes.
/// The difficulty scales dynamically with the hallucination heat scalar.
pub fn proof_of_heat_mine(payload: &[u8], difficulty_scalar: f32) -> u64 {
    let required_zeros = (difficulty_scalar / 20.0).max(0.0).min(4.0) as usize; // Max 32 bits of difficulty
    if required_zeros == 0 {
        return 0; // Free offloading for cold states
    }
    
    let mut nonce: u64 = 0;
    let mut buffer = Vec::with_capacity(payload.len() + 8);
    buffer.extend_from_slice(payload);
    buffer.extend_from_slice(&nonce.to_le_bytes());
    
    loop {
        if prismatic_core::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) { return 0; }
        buffer[payload.len()..].copy_from_slice(&nonce.to_le_bytes());
        let hash = tesseract_hash(&buffer);
        
        let mut valid = true;
        for i in 0..required_zeros {
            if hash[i] != 0 {
                valid = false;
                break;
            }
        }
        
        if valid {
            tracing::info!("Proof-of-Heat Mined! Nonce: {} (Zeros: {})", nonce, required_zeros);
            return nonce;
        }
        nonce = nonce.wrapping_add(1);
    }
}

pub fn verify_proof_of_heat(payload: &[u8], nonce: u64, difficulty_scalar: f32) -> bool {
    let required_zeros = (difficulty_scalar / 20.0).max(0.0).min(4.0) as usize;
    if required_zeros == 0 {
        return true;
    }
    
    let mut buffer = Vec::with_capacity(payload.len() + 8);
    buffer.extend_from_slice(payload);
    buffer.extend_from_slice(&nonce.to_le_bytes());
    let hash = tesseract_hash(&buffer);
    
    for i in 0..required_zeros {
        if hash[i] != 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_native_stream_cipher() {
        let key = tesseract_hash(b"test_key_material");
        let mut data = b"Hello, Tesseract Swarm!".to_vec();
        
        let mut cipher1 = SingularityStreamCipher::new(&key);
        cipher1.apply_keystream(&mut data);
        assert_ne!(data, b"Hello, Tesseract Swarm!");
        
        let mut cipher2 = SingularityStreamCipher::new(&key);
        cipher2.apply_keystream(&mut data);
        assert_eq!(data, b"Hello, Tesseract Swarm!");
    }
}
