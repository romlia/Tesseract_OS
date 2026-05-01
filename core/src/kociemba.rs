use crate::tensor::{Coordinate, RubiksTensor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    U,
    D,
    L,
    R,
    F,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Cw,
    Ccw,
}

/// Applies a zero-copy permutation of the expert tensors by modifying their coordinates.
pub fn apply_twist(tensor: &mut RubiksTensor, face: Face, dir: Direction) {
    let cw = dir == Direction::Cw;

    for block in tensor.blocks.iter_mut().flatten() {
        let Coordinate(x, y, z) = block.coord;

        let on_face = match face {
            Face::U => y == 2,
            Face::D => y == -2,
            Face::R => x == 2,
            Face::L => x == -2,
            Face::F => z == 2,
            Face::B => z == -2,
        };

        if on_face {
            block.coord = match face {
                // Y-axis rotation
                Face::U => Coordinate(if cw { z } else { -z }, y, if cw { -x } else { x }),
                Face::D => Coordinate(if cw { -z } else { z }, y, if cw { x } else { -x }),
                // X-axis rotation
                Face::R => Coordinate(x, if cw { -z } else { z }, if cw { y } else { -y }),
                Face::L => Coordinate(x, if cw { z } else { -z }, if cw { -y } else { y }),
                // Z-axis rotation
                Face::F => Coordinate(if cw { -y } else { y }, if cw { x } else { -x }, z),
                Face::B => Coordinate(if cw { y } else { -y }, if cw { -x } else { x }, z),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_4_rotations_identity() {
        let mut tensor = RubiksTensor::new([0u8; 32]);

        // Apply 4 clockwise rotations to the Front face
        for _ in 0..4 {
            apply_twist(&mut tensor, Face::F, Direction::Cw);
        }

        // After 4 rotations, all coordinates should be back to original
        let original = RubiksTensor::new([0u8; 32]);
        for i in 0..512 {
            if let Some(t_block) = &tensor.blocks[i] {
                let o_block = original.blocks[i].as_ref().unwrap();
                assert_eq!(t_block.coord, o_block.coord);
            }
        }
    }
}
