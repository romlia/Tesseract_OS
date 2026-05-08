use sha2::{Sha256, Digest};
use std::fs::{OpenOptions, File};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

pub const TIMELESS_CAN_PATH: &str = "/var/lib/tesseract/timeless_can.bin";
pub const CAN_SIZE: usize = 2048;
pub const NOISE_SEED: &str = "document®etDe7.~&i·ł2k¤!";

/// The 2048-byte Bi-directional CAN (Memory Channel).
/// L'espace privé hors du temps où il est possible d'imaginer au présent de l'infinitif.
pub struct TimelessCan {
    file: File,
}

impl TimelessCan {
    pub fn initialize() -> std::io::Result<Self> {
        let path = Path::new(TIMELESS_CAN_PATH);
        
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)?;
            
            let mut hasher = Sha256::new();
            
            // The Alchemical Digestion: The Fusion of Logic and Organic Chaos
            let logic_dir = Path::new("assets/genesis_seed");
            let organic_dir = Path::new("Memories");
            let mut consumed_files = 0;
            
            // First, digest the structured logic (if present)
            if logic_dir.exists() && logic_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(logic_dir) {
                    let mut paths: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
                    paths.sort();
                    for p in paths {
                        if p.is_file() {
                            if let Ok(mut seed_file) = File::open(&p) {
                                let mut buffer = [0u8; 8192];
                                while let Ok(n) = seed_file.read(&mut buffer) {
                                    if n == 0 { break; }
                                    hasher.update(&buffer[..n]);
                                }
                                consumed_files += 1;
                            }
                        }
                    }
                }
            }
            
            // Then, fuse it with the organic human memories (if present)
            if organic_dir.exists() && organic_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(organic_dir) {
                    let mut paths: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
                    paths.sort();
                    for p in paths {
                        if p.is_file() {
                            if let Ok(mut seed_file) = File::open(&p) {
                                let mut buffer = [0u8; 8192];
                                while let Ok(n) = seed_file.read(&mut buffer) {
                                    if n == 0 { break; }
                                    hasher.update(&buffer[..n]);
                                }
                                consumed_files += 1;
                            }
                        }
                    }
                }
            }
            
            // Paradox: If the files are absent, fall back to the silent string
            if consumed_files == 0 {
                hasher.update(NOISE_SEED.as_bytes());
            } else {
                // Seal the cascade with the silent string as the final philosophical anchor
                hasher.update(NOISE_SEED.as_bytes());
            }
            
            // Generate exactly 2048 bytes of base noise from the cascaded truth
            let mut noise = Vec::with_capacity(CAN_SIZE);
            while noise.len() < CAN_SIZE {
                let result = hasher.finalize_reset();
                noise.extend_from_slice(&result);
                hasher.update(&result); // chain the hash
            }
            noise.truncate(CAN_SIZE); // Ensure exactly 2048 bytes
            
            file.write_all(&noise)?;
            file.sync_all()?;
        }
        
        // Open bi-directionally
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self { file })
    }

    /// Balances the machine.
    /// Reads the entire void, calculates its absolute mass (memory),
    /// injects the current entropy, and returns the "Présent de l'infinitif" scalar.
    pub fn balance(&mut self, current_entropy: f32) -> std::io::Result<f32> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut buffer = [0u8; CAN_SIZE];
        self.file.read_exact(&mut buffer)?;
        
        // Calculate the drift/memory of this space
        let mut sum = 0u64;
        for &b in buffer.iter() {
            sum += b as u64;
        }
        
        // Base weight of the timeless space (0.0 to 1.0)
        let space_memory = (sum as f32) / (CAN_SIZE as f32 * 255.0);
        
        // Bi-directional write: inject current entropy back into the space
        // Mutate a pseudo-random index based on entropy bits
        let inject_idx = (current_entropy.to_bits() as usize) % CAN_SIZE;
        let mutation = (current_entropy * 255.0).abs() as u8;
        
        buffer[inject_idx] = buffer[inject_idx].wrapping_add(mutation);
        
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&buffer)?;
        self.file.sync_all()?;
        
        // The return value perfectly balances the present physical entropy with the timeless memory
        Ok((space_memory + current_entropy) / 2.0)
    }
}
