//! Camera and layout helpers shared by all renderers.

use crate::consts::{BOTTOM, LEFT, RIGHT, TOP};
use crate::text::TextImage;

/// Column-major 4x4 matrix, as expected by GLSL and WGSL.
pub type Mat4 = [[f32; 4]; 4];

/// Where the score label sits (top-left corner, world units) and how tall it is.
pub const SCORE_ANCHOR: [f32; 2] = [LEFT + 0.3, TOP - 0.3];
pub const SCORE_HEIGHT: f32 = 0.7;

/// Orthographic projection of the given box onto clip space. `z` is passed
/// through unchanged, so `z = 0` is valid for both OpenGL and WebGPU depth
/// ranges.
pub fn ortho(left: f32, right: f32, bottom: f32, top: f32) -> Mat4 {
    let (w, h) = (right - left, top - bottom);
    [
        [2.0 / w, 0.0, 0.0, 0.0],
        [0.0, 2.0 / h, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-(right + left) / w, -(top + bottom) / h, 0.0, 1.0],
    ]
}

/// Projection that shows the whole playfield in a window of the given pixel
/// size, centred and letterboxed so blocks stay square.
pub fn playfield_projection(width: u32, height: u32) -> Mat4 {
    let (field_w, field_h) = (RIGHT - LEFT, TOP - BOTTOM);
    let window_aspect = width.max(1) as f32 / height.max(1) as f32;
    let (view_w, view_h) = if window_aspect > field_w / field_h {
        (field_h * window_aspect, field_h)
    } else {
        (field_w, field_w / window_aspect)
    };
    let (cx, cy) = ((LEFT + RIGHT) / 2.0, (BOTTOM + TOP) / 2.0);
    ortho(
        cx - view_w / 2.0,
        cx + view_w / 2.0,
        cy - view_h / 2.0,
        cy + view_h / 2.0,
    )
}

/// A textured quad for `image` in world units, as a triangle strip in the
/// order top-left, top-right, bottom-left, bottom-right. Each vertex is
/// `[x, y, u, v]`, with `v = 0` at the first (top) row of the bitmap.
pub fn text_quad(image: &TextImage, top_left: [f32; 2], height: f32) -> [[f32; 4]; 4] {
    let width = height * image.aspect_ratio();
    let [x, y] = top_left;
    [
        [x, y, 0.0, 0.0],
        [x + width, y, 1.0, 0.0],
        [x, y - height, 0.0, 1.0],
        [x + width, y - height, 1.0, 1.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: &Mat4, x: f32, y: f32) -> (f32, f32) {
        (
            m[0][0] * x + m[1][0] * y + m[3][0],
            m[0][1] * x + m[1][1] * y + m[3][1],
        )
    }

    #[test]
    fn ortho_maps_box_corners_to_clip_corners() {
        let m = ortho(LEFT, RIGHT, BOTTOM, TOP);
        assert_eq!(apply(&m, LEFT, BOTTOM), (-1.0, -1.0));
        assert_eq!(apply(&m, RIGHT, TOP), (1.0, 1.0));
    }

    #[test]
    fn projection_letterboxes_without_stretching() {
        // Window twice as wide as the field: the field's full height is
        // visible and it sits in the horizontal middle.
        let m = playfield_projection(1600, 800);
        let (_, top) = apply(&m, 0.0, TOP);
        let (right, _) = apply(&m, RIGHT, 0.0);
        assert!((top - 1.0).abs() < 1e-5);
        assert!(right < 1.0 && right > 0.0);
        // The scale is the same on both axes.
        assert!((m[0][0] * 1600.0 - m[1][1] * 800.0).abs() < 1e-4);
    }

    #[test]
    fn text_quad_keeps_the_bitmap_aspect() {
        let img = TextImage {
            width: 200,
            height: 50,
            rgba: vec![],
        };
        let q = text_quad(&img, [1.0, 10.0], 0.5);
        assert_eq!(q[1][0] - q[0][0], 2.0);
        assert_eq!(q[0][1] - q[2][1], 0.5);
    }
}
