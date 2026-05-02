# Tesseract OS: Engineering Specification

This document serves as the formal engineering contract for Tesseract OS. It translates the high-level metaphors detailed in `PHILOSOPHY.md` into concrete Rust data structures, API contracts, and error-handling policies currently active in the Stage 1 & Stage 2 production baseline.

---

## 1. The Lock-Free Event Bus (The Nervous System)

All incoming human stimuli (keystrokes, raw vision frames, CPAL audio buffers) are multiplexed through the Lock-Free Event Bus before being scheduled into the WebGPU inference engine.

### Core Data Structures
```rust
pub struct CrossbeamBus<T> {
    sender: crossbeam_channel::Sender<T>,
    receiver: crossbeam_channel::Receiver<T>,
}

pub struct QueueDepthMonitor<'a, T> {
    bus: &'a dyn EventBus<T>,
    threshold_percent: usize,
}

pub enum BackpressurePolicy {
    DropOldest,
    RejectNew,
}
```

### API Contract
```rust
pub trait EventBus<T>: Send + Sync {
    /// Ingests a new event. Returns an error if the queue is at physical capacity.
    fn push(&self, event: T) -> Result<(), QueueFull>;
    
    /// Consumes the oldest event in the sequence. Returns None if the queue is empty.
    fn pop(&self) -> Option<T>;
    
    /// Returns the current number of events pending ingestion.
    fn len(&self) -> usize;
    
    /// Returns the hard-capped maximum capacity of the queue.
    fn capacity(&self) -> usize;
}
```

### Error Handling Policy: `QueueFull`
When the event horizon is breached (`push` returns `Err(QueueFull)`), the system relies on the initialized `BackpressurePolicy`:
- **`DropOldest`**: Discards the oldest sensory data to prevent heap allocation spikes. Used for volatile streams (e.g., raw audio frames where temporal decay renders old frames useless).
- **`RejectNew`**: Protects the queue by dropping the incoming frame. Used for critical state mutations where older sequences must be processed first.

---

## 2. PID-Driven Thermal Load Balancing (Thermodynamic Homeostasis)

Tesseract OS treats heat as a direct constraint on the cognitive budget. The hybrid controller ensures the system never crashes due to thermal runaway.

### Core Data Structures
```rust
// Found in the GlobalContext or mesh layer:
pub fn carnot_efficiency(t_hot: f32, t_cold: f32) -> f32 {
    if t_hot <= t_cold { return 0.0; }
    1.0 - (t_cold / t_hot)
}
```

### API Contract
```rust
/// Derives a cryptographic proof-of-work based on the physical thermodynamic cost.
pub fn proof_of_heat_mine(_payload: &[u8], _difficulty_scalar: f32) -> u64;

/// Verifies that a given computational payload has paid its thermodynamic debt.
pub fn verify_proof_of_heat(_payload: &[u8], _nonce: u64, _difficulty_scalar: f32) -> bool;
```

### Error Handling Policy
If `carnot_efficiency` drops below the hard-coded baseline, or if a payload fails `verify_proof_of_heat`, the payload is **rejected instantly** and dropped from the `EventBus`. There are no retries for payload submission if the thermal envelope is violated, ensuring the CPU/GPU is not choked by infinite retry loops.

---

## 3. Cryptographic PKI (The Universal Sovereign)

The zero-trust framework relies on Ed25519 signatures to verify the origin of all data crossing the swarm, completely eliminating IP-based trust.

### Core Data Structures
```rust
pub struct NodeTrustStore {
    trusted_nodes: std::collections::HashMap<String, ed25519_dalek::VerifyingKey>,
}

pub struct SingularityStreamCipher {
    cipher: chacha20::ChaCha20,
}
```

### API Contract
```rust
impl NodeTrustStore {
    /// Caches a verified sovereign public key into memory.
    pub fn add_trusted_node(&mut self, node_id: String, public_key: VerifyingKey);
    
    /// Verifies the cryptographic integrity of an incoming mesh payload against the cached Key.
    pub fn verify_swarm_payload(&self, node_id: &str, payload: &[u8], signature_bytes: &[u8; 64]) -> bool;
}

impl SingularityStreamCipher {
    /// Zero-allocation stream cipher operation.
    pub fn apply_keystream(&mut self, data: &mut [u8]);
}
```

### Error Handling Policy
Any payload failing `verify_swarm_payload` is immediately dropped from the `MeshPacket` queue. The failure event is logged silently to the diagnostic buffer; no TCP NACK or error response is sent over the network to prevent DDoS amplification attacks.

---

## 4. The Yin-Yang Membrane & Biometric Staking (The Gateway)

Data strictly resides in the Private Sphere until explicitly sanctioned by human biological entropy to enter the Public Sphere (The Hive).

### Core Data Structures
```rust
pub enum ExecutionIntent {
    PublishThought,
    ModifyState,
    RequestResource,
}

pub struct ZeroTrustLedger {
    entries: Vec<[u8; 32]>,
}

pub struct HardwareEnclave {
    pub is_active: bool,
}

pub struct YinYangMembrane;
```

### API Contract
```rust
impl YinYangMembrane {
    /// Requires active biological entropy (e.g., from an RF or audio source) to generate a staking key.
    pub fn get_staking_entropy(&self, entropy_pool: &[u8]) -> [u8; 32];
    
    /// Formally moves subjective private data into objective public truth.
    pub fn crystallize(ledger: &mut ZeroTrustLedger, private_freewheel: &[f32], public_truth: &mut [f32]) -> bool;
}

impl HardwareEnclave {
    /// Locks the memory region from user-space observation.
    pub fn protect_memory_region(&self, _ptr: *const u8, _len: usize);
}
```

### Error Handling Policy
If the biological entropy pool is exhausted or insufficient (e.g. `get_staking_entropy` fails to gather enough entropy), the `crystallize` function will return `false` and the state remains locked in the `PrivateInferenceEngine`. The user must physically interact with the machine (typing, speaking, moving the mouse) to generate the requisite ambient noise for the operation to proceed.
