//! CPU text rasterisation for the score display.
//!
//! The score changes rarely, so rendering it to a small RGBA bitmap on the
//! CPU and uploading that as a texture is both simpler and cheaper than a
//! per-frame GPU text pipeline: the frontends re-rasterise only when the
//! string changes and draw a single textured quad otherwise.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};

/// Bundled UI font.
pub fn default_font() -> FontRef<'static> {
    FontRef::try_from_slice(include_bytes!("../res/fonts/Roboto-Regular.ttf"))
        .expect("bundled font is valid")
}

/// A white-on-transparent RGBA8 bitmap, rows stored top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl TextImage {
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// Rasterise a single line of text at the given pixel height.
pub fn rasterize(font: &impl Font, text: &str, px: f32) -> TextImage {
    let scaled = font.as_scaled(PxScale::from(px));
    let ascent = scaled.ascent();
    let height = (ascent - scaled.descent()).ceil().max(1.0) as u32;

    let mut glyphs = Vec::with_capacity(text.len());
    let mut caret = 0.0;
    let mut prev = None;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(prev) = prev {
            caret += scaled.kern(prev, id);
        }
        glyphs.push(id.with_scale_and_position(px, point(caret, ascent)));
        caret += scaled.h_advance(id);
        prev = Some(id);
    }
    let width = caret.ceil().max(1.0) as u32;

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for glyph in glyphs {
        let Some(outline) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outline.px_bounds();
        outline.draw(|x, y, coverage| {
            let px = bounds.min.x as i32 + x as i32;
            let py = bounds.min.y as i32 + y as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                return;
            }
            let i = 4 * (py as usize * width as usize + px as usize);
            let alpha = (coverage * 255.0).round() as u8;
            rgba[i..i + 3].fill(255);
            rgba[i + 3] = rgba[i + 3].max(alpha);
        });
    }
    TextImage {
        width,
        height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_visible_pixels() {
        let img = rasterize(&default_font(), "Score: 10", 32.0);
        assert!(img.width > 50 && img.height >= 32);
        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
        let opaque = img.rgba.chunks(4).filter(|p| p[3] > 128).count();
        assert!(opaque > 100, "expected some opaque pixels, got {opaque}");
    }

    #[test]
    fn empty_string_yields_a_valid_bitmap() {
        let img = rasterize(&default_font(), "", 16.0);
        assert!(img.width >= 1 && img.height >= 1);
    }
}
