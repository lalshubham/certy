use crate::config::FONT_SIZE;
use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;

pub struct CachedGlyph {
    pub metrics: Metrics,
    pub bitmap: Vec<u8>,
}

pub struct FontManager {
    font: Font,
    cache: HashMap<char, CachedGlyph>,
    pub char_width: usize,
    pub line_height: usize,
    pub baseline_offset: usize,
}

impl FontManager {
    pub fn new() -> Self {
        let paths = [
            "/usr/share/fonts/google-noto-sans-mono-fonts/NotoSansMono-Regular.ttf",
            "/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf",
            "/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf",
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        ];

        let bytes = paths
            .iter()
            .find_map(|path| std::fs::read(path).ok())
            .expect("No monospace font found on system.");

        let font = Font::from_bytes(bytes, FontSettings::default()).expect("Invalid font data");
        let metrics = font
            .horizontal_line_metrics(FONT_SIZE)
            .expect("Missing horizontal metrics");

        let line_height = metrics.new_line_size.ceil() as usize;
        let baseline_offset = metrics.ascent.ceil() as usize;

        let mut cache = HashMap::with_capacity(128);
        for byte in 32u8..=126u8 {
            let ch = byte as char;
            let (m, bitmap) = font.rasterize(ch, FONT_SIZE);
            cache.insert(ch, CachedGlyph { metrics: m, bitmap });
        }

        let char_width = cache.get(&'M').unwrap().metrics.advance_width.ceil() as usize;

        Self {
            font,
            cache,
            char_width,
            line_height,
            baseline_offset,
        }
    }

    pub fn draw_char(
        &mut self,
        buf: &mut [u32],
        ch: char,
        x: usize,
        y: usize,
        screen_w: usize,
        screen_h: usize,
        color: u32,
    ) {
        let glyph = self.cache.entry(ch).or_insert_with(|| {
            let (metrics, bitmap) = self.font.rasterize(ch, FONT_SIZE);
            CachedGlyph { metrics, bitmap }
        });

        if glyph.metrics.width == 0 || glyph.metrics.height == 0 {
            return;
        }

        let top =
            (y + self.baseline_offset) as i32 - (glyph.metrics.height as i32 + glyph.metrics.ymin);
        let left = x as i32 + glyph.metrics.xmin;

        for row in 0..glyph.metrics.height {
            let py = top + row as i32;
            if py < 0 || py as usize >= screen_h {
                continue;
            }

            for col in 0..glyph.metrics.width {
                let px = left + col as i32;
                if px < 0 || px as usize >= screen_w {
                    continue;
                }

                let alpha = glyph.bitmap[row * glyph.metrics.width + col];
                if alpha == 0 {
                    continue;
                }

                let idx = py as usize * screen_w + px as usize;
                buf[idx] = blend_alpha(buf[idx], color, alpha);
            }
        }
    }
}

#[inline(always)]
fn blend_alpha(bg: u32, fg: u32, alpha: u8) -> u32 {
    if alpha == 255 {
        return fg;
    }
    let a = alpha as u32;
    let inv = 255 - a;

    let r = (((fg >> 16) & 0xFF) * a + ((bg >> 16) & 0xFF) * inv) / 255;
    let g = (((fg >> 8) & 0xFF) * a + ((bg >> 8) & 0xFF) * inv) / 255;
    let b = ((fg & 0xFF) * a + (bg & 0xFF) * inv) / 255;

    0xFF000000 | (r << 16) | (g << 8) | b
}
