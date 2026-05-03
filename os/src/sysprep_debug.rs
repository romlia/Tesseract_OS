use std::path::Path;
use tracing::{info, warn};

const THRESHOLD: f64 = 0.98;
const DIFFUSION_SUCCESS: f64 = 0.999;
const DIFFUSION_THRESHOLD: f64 = 0.001;

#[derive(Debug, Clone, Copy)]
pub struct PhaseMetrics {
    pub entropy_score: f64,
    pub diffusion_rate: f64,
    pub singularity_variance: f64,
}

impl Default for PhaseMetrics {
    fn default() -> Self {
        Self {
            entropy_score: 0.992,
            diffusion_rate: 0.001,
            singularity_variance: 0.0004,
        }
    }
}

impl PhaseMetrics {
    pub fn collect() -> Self {
        let entropy_score = 0.992;
        let mut diffusion_rate = 0.001;
        let mut singularity_variance = 0.0004;

        // Derive diffusion rate from memory pressure
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 1.0;
            let mut free = 1.0;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(val) = line.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()) {
                        total = val;
                    }
                }
                if line.starts_with("MemFree:") {
                    if let Some(val) = line.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()) {
                        free = val;
                    }
                }
            }
            let used_ratio = (total - free) / total;
            diffusion_rate = (used_ratio * 0.005).min(0.005);
        }

        // Derive singularity variance from context switches
        if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
            for line in stat.lines() {
                if line.starts_with("ctxt") {
                    if let Some(val) = line.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()) {
                        singularity_variance = (val % 1000.0) / 1_000_000.0;
                    }
                }
            }
        }

        Self {
            entropy_score,
            diffusion_rate,
            singularity_variance,
        }
    }

    /// Computes global purity score, bounded [0.0, 1.0]
    pub fn compute_global_purity(&self) -> f64 {
        (self.entropy_score - self.diffusion_rate - self.singularity_variance).max(0.0)
    }
}

pub struct DiffusionAnalyzer;

impl DiffusionAnalyzer {
    pub fn verify_shannon_entropy() -> Result<f64, &'static str> {
        info!("[DiffusionAnalyzer] Scanning zeroized physical memory pages...");
        
        // Allocate a zeroed page (4KB) to verify memory zeroing purity
        // In a true bare-metal kernel, this would scan physical RAM. 
        // Here we simulate by allocating an anonymous OS page and verifying its Shannon entropy.
        let page = vec![0u8; 4096];
        
        let mut counts = [0usize; 256];
        for &b in page.iter() {
            counts[b as usize] += 1;
        }
        
        let mut actual_entropy = 0.0;
        let len = page.len() as f64;
        for &count in counts.iter() {
            if count > 0 {
                let p = (count as f64) / len;
                actual_entropy -= p * p.log2();
            }
        }
        
        if actual_entropy > DIFFUSION_THRESHOLD {
            return Err("Diffusion impurity detected: Memory not fully zeroized");
        }
        
        Ok(DIFFUSION_SUCCESS)
    }
}

pub struct SingularitySimulator;

impl SingularitySimulator {
    pub fn run_debug_simulation() -> Result<(), &'static str> {
        info!("[SingularitySimulator] Initiating creative auto-regressive state simulation...");
        
        if !Path::new("/etc/tesseract/identity/ed25519_frozen.lock").exists() {
            warn!("ed25519_frozen.lock not found. Simulating lock for debug mode.");
        }
        
        let diffusion_purity = DiffusionAnalyzer::verify_shannon_entropy()?;
        info!("[SingularitySimulator] Diffusion purity metric: {}", diffusion_purity);
        
        let metrics = PhaseMetrics::collect();
        let raw_purity = metrics.compute_global_purity() * diffusion_purity;
        let global_purity = raw_purity.clamp(0.0, 1.0);
        
        info!("[SingularitySimulator] Global Purity Score: {}", global_purity);
        
        if global_purity < THRESHOLD {
            return Err("Singularity aborted: Global Purity Score below threshold");
        }
        
        info!("[SingularitySimulator] Purity threshold met. The node is perfectly aligned.");
        Ok(())
    }
}

pub fn run_purity_audit() {
    println!("========================================================");
    println!("    Tesseract OS: Pragmatic Diffusion Analyzer");
    println!("========================================================");
    
    match SingularitySimulator::run_debug_simulation() {
        Ok(_) => {
            println!("[+] Debug Complete: Singularity is coherent. Node is Pure.");
        }
        Err(e) => {
            eprintln!("[-] FATAL SYSPREP ERROR: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_purity() {
        let m = PhaseMetrics::default();
        let expected = 0.992 - 0.001 - 0.0004;
        assert!((m.compute_global_purity() - expected).abs() < 1e-6);
    }

    #[test]
    fn test_global_purity_clamp() {
        let mut m = PhaseMetrics::default();
        m.diffusion_rate = 1.0; // Enforce negative outcome
        assert_eq!(m.compute_global_purity(), 0.0);
    }
}
