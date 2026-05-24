// src/lib.rs
pub mod animate_atlas;
pub mod sparrow_atlas;

// Re-export core data structures for clean root access
pub use animate_atlas::{draw_part_mesh, get_animate_parts, parse_animate_atlas, AnimateAtlas};
pub use sparrow_atlas::{parse_sparrow, SparrowFrame};
