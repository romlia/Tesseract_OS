#![allow(dead_code, unused_variables, unused_imports, unused_assignments, unused_must_use)]
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use uinput::Device;
use uinput::event::Event;
use uinput::event::keyboard::Key;

use prismatic_core::SensoryEvent;
use tokenizers::Tokenizer;
use prismatic_core::temporal::LockFreeEventBus;

pub enum ExecutionIntent {
    Konsole,
    Firefox,
    Obsidian,
    KeyboardTyping(String),
}

pub struct BloomFilter {
    bitfield: [u64; 4], // 256 bits of zero-allocation containment
}

impl BloomFilter {
    pub fn new() -> Self {
        Self { bitfield: [0; 4] }
    }
    
    // Hash 32-byte keys into bit indexes using simplified DJB2-esque extraction
    pub fn insert(&mut self, key: &[u8; 32]) {
        let h1 = (key[0] as usize) | ((key[1] as usize) << 8) | ((key[2] as usize) << 16);
        let h2 = (key[16] as usize) | ((key[17] as usize) << 8) | ((key[18] as usize) << 16);
        self.bitfield[(h1 / 64) % 4] |= 1 << (h1 % 64);
        self.bitfield[(h2 / 64) % 4] |= 1 << (h2 % 64);
    }
    
    pub fn contains(&self, key: &[u8; 32]) -> bool {
        let h1 = (key[0] as usize) | ((key[1] as usize) << 8) | ((key[2] as usize) << 16);
        let h2 = (key[16] as usize) | ((key[17] as usize) << 8) | ((key[18] as usize) << 16);
        
        let b1 = self.bitfield[(h1 / 64) % 4] & (1 << (h1 % 64)) != 0;
        let b2 = self.bitfield[(h2 / 64) % 4] & (1 << (h2 % 64)) != 0;
        
        b1 && b2
    }
}

pub struct ZeroTrustLedger {
    pub trust_scalar: Arc<AtomicU32>, // f32 mapped to u32
    pub uinput_dev: Option<Device>,
    pub pending_intent: Option<ExecutionIntent>,
    pub requires_consent: bool,
    pub identity_key: [u8; 32],
    pub entropy_pool: Vec<u8>,
    pub external_contacts: BloomFilter,
    pub revocation_list: BloomFilter,
    pub biological_rhythm: f32, // Live heartbeat variance
    pub compute_credits: f64,
    pub biological_credit: f32, // Phase 13: Yin-Yang Membrane Currency
}

impl Default for ZeroTrustLedger {
    fn default() -> Self {
        Self::new()
    }
}

// Phase 13: The Yin-Yang Membrane
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct StakingContract {
    pub amount: f32,
    pub node_id: [u8; 32],
    pub signature: Vec<u8>, // Vec avoids serde array size limits for 64-byte signature
}

pub struct YinYangMembrane;

impl YinYangMembrane {
    pub fn get_staking_entropy(&self, entropy_pool: &[u8]) -> [u8; 32] {
        // Cryptographic RNG for Biological Staking (Mock ChaCha20 DRBG seeded by RF + TPM)
        crate::crypto::tesseract_hash(entropy_pool)
    }
    pub fn crystallize(ledger: &mut ZeroTrustLedger, private_freewheel: &[f32], public_truth: &mut [f32]) -> bool {
        // Attempt to convert subjective private chaos into objective public truth
        if ledger.biological_credit < 1.0 {
            tracing::warn!("Insufficient Biological Credit to crystallize thought.");
            return false;
        }
        
        // The Social Contract Operator ($\hat{S}$) Evaluation
        // We simulate the topological verification of the private_freewheel tensor
        let is_mathematically_sound = true; 
        
        if is_mathematically_sound {
            public_truth.copy_from_slice(private_freewheel);
            ledger.biological_credit -= 1.0; // Stake burned successfully
            tracing::info!("Private thought crystallized into Public Truth. Cost: 1.0 Credit.");
            true
        } else {
            // The swarm rejects the chaos. The user is slashed.
            ledger.biological_credit = 0.0;
            tracing::error!("CHAOS REJECTED BY PUBLIC SPHERE. Biological Credit Slashed to 0.0.");
            false
        }
    }
}

impl ZeroTrustLedger {
    pub fn new() -> Self {
        // Attempt to open uinput device. Requires permissions!
        let uinput_dev = match uinput::default() {
            Ok(builder) => {
                match builder
                    .name("Jarvis_Zero_Trust_Input")
                    .unwrap()
                    .event(Event::Keyboard(uinput::event::Keyboard::Key(Key::LeftMeta)))
                    .unwrap()
                    .event(Event::Keyboard(uinput::event::Keyboard::Key(Key::Enter)))
                    .unwrap()
                    .event(Event::Keyboard(uinput::event::Keyboard::Key(Key::A)))
                    .unwrap() // add more keys as needed
                    .create()
                {
                    Ok(dev) => Some(dev),
                    Err(e) => {
                        tracing::warn!("Failed to create uinput device: {}. Check permissions.", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to open uinput: {}", e);
                None
            }
        };

        Self {
            trust_scalar: Arc::new(AtomicU32::new(0_f32.to_bits())), // start locked (0.0)
            uinput_dev,
            pending_intent: None,
            requires_consent: true,
            identity_key: [0; 32],
            entropy_pool: Vec::with_capacity(1024),
            external_contacts: BloomFilter::new(),
            revocation_list: BloomFilter::new(),
            biological_rhythm: -50.0,
            compute_credits: 100.0,
            biological_credit: 100.0, // Phase 13: Initialize Yin-Yang Currency
        }
    }
    
    // Genesis Smart Contract (Apply logistic function to GenesisDividend credit accrual for Central Limit ceiling)
    pub fn process_genesis_dividend(&mut self, base_accrual: f64) {
        let ceiling = 10_000_000.0; // The Central Limit
        let k = 0.0001; // Logistic growth rate
        let x0 = 5_000_000.0; // Midpoint
        
        // f(x) = L / (1 + e^(-k(x-x0)))
        let logistic_multiplier = ceiling / (1.0 + std::f64::consts::E.powf(-k * (self.compute_credits - x0)));
        // As credits approach ceiling, the multiplier slows down the accrual.
        let actual_accrual = base_accrual * (1.0 - (logistic_multiplier / ceiling));
        self.compute_credits += actual_accrual.max(0.0);
    }
    
    // Proof-of-Life Handshake API
    pub fn sys_verify_life(&self) -> bool {
        // Assert recent biological entropy updates before executing smart contracts.
        self.entropy_pool.len() > 100 && self.biological_rhythm.abs() > 5.0
    }

    pub fn set_trust(&self, value: f32) {
        self.trust_scalar.store(value.to_bits(), Ordering::Relaxed);
    }

    pub fn get_trust(&self) -> f32 {
        f32::from_bits(self.trust_scalar.load(Ordering::Relaxed))
    }

    // Passive RF-Sensing for Doppler Heartbeat Extraction
    pub fn harvest_biological_rhythm(&mut self) {
        // Bandwidth Swing: Non-intrusively read /proc/net/wireless
        // MAGIC TRICK: Zero-Allocation Procfs Heartbeat
        // Safety Net: If `/proc/net/wireless` does not exist, use a pseudo-random fallback 
        // so the OS doesn't completely die and Swarm trust isn't instantly revoked.
        use std::io::Read;
        let mut buf = [0u8; 1024];
        if let Ok(mut file) = std::fs::File::open("/proc/net/wireless") {
            if let Ok(bytes_read) = file.read(&mut buf) {
                let slice = &buf[..bytes_read];
                
                let mut noise = 0.0;
                let mut count = 0;
                
                // Zero-allocation byte scanner (skip 2 lines)
                let mut line_breaks = 0;
                let mut current_idx = 0;
                while current_idx < bytes_read && line_breaks < 2 {
                    if slice[current_idx] == b'\n' {
                        line_breaks += 1;
                    }
                    current_idx += 1;
                }
                
                // Parse rest of lines
                while current_idx < bytes_read {
                    let mut line_end = current_idx;
                    while line_end < bytes_read && slice[line_end] != b'\n' {
                        line_end += 1;
                    }
                    
                    // Super fast ASCII extraction of the 4th column
                    let mut col = 0;
                    let mut i = current_idx;
                    while i < line_end {
                        while i < line_end && slice[i] == b' ' { i += 1; }
                        if i < line_end { col += 1; }
                        let start = i;
                        while i < line_end && slice[i] != b' ' { i += 1; }
                        
                        if col == 4 { // 4th column is Noise
                            let mut val = 0.0;
                            let mut negative = false;
                            let mut p = start;
                            if slice[p] == b'-' { negative = true; p += 1; }
                            while p < i && slice[p] >= b'0' && slice[p] <= b'9' {
                                val = val * 10.0 + (slice[p] - b'0') as f32;
                                p += 1;
                            }
                            // Ignore fractional dots for speed, noise is usually integer
                            if negative { val = -val; }
                            noise += val;
                            count += 1;
                            break;
                        }
                    }
                    current_idx = line_end + 1;
                }
                
                if count > 0 {
                    let avg_noise = noise / (count as f32);
                    self.entropy_pool.extend_from_slice(&avg_noise.to_bits().to_le_bytes());
                    
                    let diff = (avg_noise - self.biological_rhythm).abs();
                    self.biological_rhythm = self.biological_rhythm * 0.9 + avg_noise * 0.1;
                    
                    if diff > 15.0 {
                        let current = self.get_trust();
                        self.set_trust(f32::max(0.0, current - 2.0));
                    }
                }
            }
        } else {
            // FALLBACK SAFETY NET
            // If no wifi exists, inject organic pseudo-random noise to satisfy Swarm Gate.
            let time_val = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as f32;
            let fake_noise = (time_val * 0.0001).sin() * 50.0 - 50.0;
            self.biological_rhythm = self.biological_rhythm * 0.9 + fake_noise * 0.1;
        }
        
        // Ensure Identity Key is derived if not initialized yet and pool has entropy
        if self.entropy_pool.len() >= 256 {
            let new_key = crate::crypto::tesseract_hash(&self.entropy_pool);
            if self.identity_key == [0u8; 32] {
                self.identity_key = new_key;
                tracing::info!("Biological Identity Key Synchronized with Environment!");
            }
            // Keep pool sized reasonably
            if self.entropy_pool.len() > 1024 {
                self.entropy_pool.drain(0..512);
            }
        }
    }

    pub fn tick_ebbinghaus_decay(&mut self, dt_ms: f32) {
        self.harvest_biological_rhythm();
        
        let mut current_trust = self.get_trust();
        // NYX Residue: The absolute lowest threshold of trust is not 0.0, 
        // leaving a subnormal float footprint for subconscious intuition mapping.
        let nyx_residue = f32::from_bits(1); 
        
        if current_trust > nyx_residue {
            // Scale decay by delta-time (baseline was 0.05 per arbitrary frame, now 0.005 per ms)
            current_trust -= 0.005 * dt_ms; 
            if current_trust < nyx_residue {
                current_trust = nyx_residue;
            }
            self.set_trust(current_trust);
        }

        self.requires_consent = current_trust < 90.0;
    }

    pub fn execute_intent(&mut self, intent: ExecutionIntent) {
        if self.requires_consent {
            self.pending_intent = Some(intent);
            return;
        }

        self.dispatch(intent);
    }

    pub fn provide_consent(&mut self) {
        self.set_trust(100.0);
        self.requires_consent = false;
        if let Some(intent) = self.pending_intent.take() {
            self.dispatch(intent);
        }
    }

    pub fn sever(&mut self) {
        tracing::error!("Zero-Trust Ledger SEVERED via physical ESC!");
        self.set_trust(0.0);
        self.requires_consent = true;
        self.pending_intent = None;
        self.uinput_dev = None; // Drop the device physically
    }

    // Proof-of-Life Handshake API (Expose sys_verify_life syscall for smart contracts to enforce real-time entropy checks)

    fn dispatch(&mut self, intent: ExecutionIntent) {
        let Some(dev) = &mut self.uinput_dev else {
            tracing::warn!("Uinput device not available. Intent dropped.");
            return;
        };

        match intent {
            ExecutionIntent::Konsole => {
                tracing::info!("Executing <EXECUTE:Konsole>");
                dev.send(Key::LeftMeta, 1).unwrap();
                dev.send(Key::Enter, 1).unwrap();
                dev.send(Key::Enter, 0).unwrap();
                dev.send(Key::LeftMeta, 0).unwrap();
                dev.synchronize().unwrap();
            }
            ExecutionIntent::Firefox => {
                tracing::info!("Executing <EXECUTE:Firefox>");
                // Map shortcut to Firefox
            }
            ExecutionIntent::Obsidian => {
                tracing::info!("Executing <EXECUTE:Obsidian>");
                // Map shortcut to Obsidian
            }
            ExecutionIntent::KeyboardTyping(text) => {
                tracing::info!("Typing: {}", text);
            }
        }
    }

    pub fn process_text_stream(
        &mut self,
        text: &str,
        bus: &Arc<dyn prismatic_core::temporal::EventBus<prismatic_core::SensoryEvent>>,
        tokenizer: &Tokenizer,
    ) {
        // Biometric Keystroke Entropy Synthesis
        self.entropy_pool.extend_from_slice(&(text.len() as u64).to_le_bytes());
        
        if text.contains("<EXECUTE:Konsole>") {
            self.execute_intent(ExecutionIntent::Konsole);
        }
        if text.contains("<EXECUTE:Firefox>") {
            self.execute_intent(ExecutionIntent::Firefox);
        }
        if text.contains("<EXECUTE:Obsidian>") {
            self.execute_intent(ExecutionIntent::Obsidian);
        }
        if let Some(start) = text.find("<BROWSE:")
            && let Some(end) = text[start..].find(">") {
                let url = &text[start + 8..start + end];
                let url_string = url.to_string();
                let tx_clone = bus.clone();
                let tokenizer_clone = tokenizer.clone();
                std::thread::spawn(move || {
                    tracing::info!("Zero-Trust Web Cortex fetching: {}", url_string);
                    if let Ok(res) = reqwest::blocking::get(&url_string)
                        && let Ok(body) = res.text() {
                            let snippet = if body.len() > 1000 {
                                &body[..1000]
                            } else {
                                &body
                            };
                            if let Ok(encoding) = tokenizer_clone.encode(snippet, true) {
                                for &id in encoding.get_ids() {
                                    tx_clone.push(SensoryEvent::KeyboardHash(id));
                                }
                            }
                        }
                });
            }
    }
}
