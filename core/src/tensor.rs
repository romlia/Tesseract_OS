#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinate(pub i8, pub i8, pub i8); // x, y, z in [-2, -1, 0, 1, 2]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorPrecision {
    Ternary1_58, // Core (Inner 3x3x3 blocks)
    Float32,     // Experts (Outer 5x5x5 shell blocks)
}

#[derive(Debug, Clone)]
pub struct RubiksBlock {
    pub coord: Coordinate,
    pub precision: TensorPrecision,
    pub data_ptr: usize, 

}

/// Z-Order Morton Encoding for O(1) Matrix Scaling
pub fn morton_encode(x: i8, y: i8, z: i8) -> usize {
    // Map [-2, 2] to [0, 4]
    let ux = (x + 2) as u32;
    let uy = (y + 2) as u32;
    let uz = (z + 2) as u32;
    
    let mut morton = 0;
    for i in 0..3 { // 3 bits per coordinate
        let bit_x = (ux >> i) & 1;
        let bit_y = (uy >> i) & 1;
        let bit_z = (uz >> i) & 1;
        
        morton |= bit_x << (3 * i);
        morton |= bit_y << (3 * i + 1);
        morton |= bit_z << (3 * i + 2);
    }
    
    morton as usize
}

#[derive(Debug)]
pub struct RubiksTensor {
    // Flat 1D array of 512 elements mapped via Morton Codes, enabling O(1) memory lookup
    // Even with Morton Codes, reading disparate blocks can cause cache misses.
    // By invoking `core::arch::x86_64::_mm_prefetch`, we can explicitly instruct the CPU 
    // to load the next mathematical block into the L1 cache *before* the loop requests it,
    // essentially reaching 0ns memory read latency.
    pub blocks: Vec<Option<RubiksBlock>>,
}

impl RubiksTensor {
    pub fn new(seed: [u8; 32]) -> Self {
        let mut blocks = vec![None; 512];
        
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        for z in -2..=2 {
            for y in -2..=2 {
                for x in -2..=2 {
                    let mut hasher = DefaultHasher::new();
                    seed.hash(&mut hasher);
                    x.hash(&mut hasher);
                    y.hash(&mut hasher);
                    z.hash(&mut hasher);
                    
                    if hasher.finish().is_multiple_of(10) {
                        continue; // Cull ~10% of pathways
                    }
                    
                    let is_shell = x == 2 || x == -2 || y == 2 || y == -2 || z == 2 || z == -2;
                    let precision = if is_shell {
                        TensorPrecision::Float32
                    } else {
                        TensorPrecision::Ternary1_58
                    };
                    
                    let idx = morton_encode(x, y, z);
                    blocks[idx] = Some(RubiksBlock {
                        coord: Coordinate(x, y, z),
                        precision,
                        data_ptr: 0,
                    });
                }
            }
        }
        
        Self { blocks }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_geometry() {
        let tensor = RubiksTensor::new([0u8; 32]);
        assert_eq!(tensor.blocks.len(), 512);
        assert!(tensor.blocks.iter().flatten().count() <= 125);

        let core = tensor
            .blocks
            .iter()
            .flatten()
            .find(|b| b.coord == Coordinate(0, 0, 0))
            .unwrap();
        assert_eq!(core.precision, TensorPrecision::Ternary1_58);

        let corner = tensor
            .blocks
            .iter()
            .flatten()
            .find(|b| b.coord == Coordinate(2, 2, 2))
            .unwrap();
        assert_eq!(corner.precision, TensorPrecision::Float32);
    }
}
