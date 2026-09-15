//! Geometry of a single block, shared by every renderer.
//!
//! All blocks are the same rounded square, so a renderer uploads this outline
//! once and draws every block as an instance of it (translated, rotated and
//! tinted per instance).

use crate::consts::{BLOCK_SIZE, CORNER_RADIUS, EDGES_PER_CORNER, VERTS_PER_BLOCK};

/// Outline of a rounded square centred on the origin, counter-clockwise,
/// starting in the top-right corner. Produces `4 * (edges_per_corner + 1)`
/// points.
pub fn rounded_square(half_size: f32, radius: f32, edges_per_corner: u32) -> Vec<[f32; 2]> {
    let inner = half_size - radius;
    let corner: Vec<[f32; 2]> = (0..=edges_per_corner)
        .map(|n| {
            let angle = n as f32 * std::f32::consts::FRAC_PI_2 / edges_per_corner as f32;
            [inner + radius * angle.cos(), inner + radius * angle.sin()]
        })
        .collect();
    let mut pts = corner.clone();
    pts.extend(corner.iter().map(|&[x, y]| [-y, x]));
    pts.extend(corner.iter().map(|&[x, y]| [-x, -y]));
    pts.extend(corner.iter().map(|&[x, y]| [y, -x]));
    pts
}

/// The outline of one game block.
pub fn block_outline() -> Vec<[f32; 2]> {
    rounded_square(BLOCK_SIZE / 2.0, CORNER_RADIUS, EDGES_PER_CORNER)
}

/// Triangle-list indices that fan-triangulate a convex outline of
/// [`VERTS_PER_BLOCK`] vertices.
pub fn block_indices() -> Vec<u16> {
    (1..VERTS_PER_BLOCK as u16 - 1)
        .flat_map(|i| [0, i, i + 1])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_has_expected_vertex_count_and_is_convex() {
        let pts = block_outline();
        assert_eq!(pts.len(), VERTS_PER_BLOCK as usize);
        // Every consecutive edge pair turns the same way (counter-clockwise).
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            let c = pts[(i + 2) % pts.len()];
            let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            assert!(cross > 0.0, "edge {i} is not counter-clockwise");
        }
    }

    #[test]
    fn indices_cover_all_vertices() {
        let idx = block_indices();
        assert_eq!(idx.len(), 3 * (VERTS_PER_BLOCK as usize - 2));
        assert_eq!(*idx.iter().max().unwrap(), VERTS_PER_BLOCK as u16 - 1);
    }
}
