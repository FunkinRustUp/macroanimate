// src/lib.rs
pub mod sparrow_atlas;
pub mod texture_atlas;

// Re-export core data structures for clean root access
pub use sparrow_atlas::{SparrowFrame, parse_sparrow};
pub use texture_atlas::{AnimateAtlas, draw_part_mesh, get_animate_parts, parse_animate_atlas};
