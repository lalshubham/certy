use super::font::FontManager;

#[inline(always)]
pub fn draw_solid_rect(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    let start_y = y.min(screen_h);
    let end_y = (y + h).min(screen_h);
    let start_x = x.min(screen_w);
    let end_x = (x + w).min(screen_w);
    if start_x >= end_x {
        return;
    }
    for row in start_y..end_y {
        let offset = row * screen_w;
        buf[offset + start_x..offset + end_x].fill(color);
    }
}

#[inline(always)]
pub fn draw_solid_rect_i32(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    color: u32,
) {
    if x >= screen_w as i32 || y >= screen_h as i32 {
        return;
    }
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w as i32).max(0) as usize).min(screen_w);
    let y1 = ((y + h as i32).max(0) as usize).min(screen_h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for row in y0..y1 {
        let offset = row * screen_w;
        buf[offset + x0..offset + x1].fill(color);
    }
}

#[inline(always)]
pub fn draw_solid_rect_clipped(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    clip_min_x: usize,
    clip_max_x: usize,
    color: u32,
) {
    if y >= screen_h as i32 || x >= clip_max_x as i32 {
        return;
    }
    let x0 = (x.max(clip_min_x as i32) as usize).min(clip_max_x);
    let x1 = ((x + w as i32).max(clip_min_x as i32) as usize).min(clip_max_x);
    let y0 = y.max(0) as usize;
    let y1 = ((y + h as i32).max(0) as usize).min(screen_h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for row in y0..y1 {
        let offset = row * screen_w;
        buf[offset + x0..offset + x1].fill(color);
    }
}

#[inline(always)]
pub fn draw_string(
    fonts: &mut FontManager,
    frame: &mut [u32],
    text: &str,
    start_x: i32,
    start_y: i32,
    screen_w: usize,
    screen_h: usize,
    color: u32,
) {
    let mut x = start_x;
    let cw = fonts.char_width as i32;
    for ch in text.chars() {
        if x >= screen_w as i32 {
            break;
        }
        fonts.draw_char(frame, ch, x, start_y, screen_w, screen_h, color);
        x += cw;
    }
}

#[inline(always)]
pub fn draw_string_clipped(
    fonts: &mut FontManager,
    frame: &mut [u32],
    text: &str,
    start_x: i32,
    start_y: i32,
    clip_min_x: usize,
    clip_max_x: usize,
    screen_w: usize,
    screen_h: usize,
    color: u32,
) {
    let mut x = start_x;
    let cw = fonts.char_width as i32;
    for ch in text.chars() {
        if x >= clip_min_x as i32 && x + cw <= clip_max_x as i32 {
            fonts.draw_char(frame, ch, x, start_y, screen_w, screen_h, color);
        }
        x += cw;
        if x >= clip_max_x as i32 {
            break;
        }
    }
}

#[inline(always)]
pub fn draw_string_ellipsis(
    fonts: &mut FontManager,
    frame: &mut [u32],
    text: &str,
    start_x: i32,
    start_y: i32,
    max_x: usize,
    screen_w: usize,
    screen_h: usize,
    color: u32,
) {
    if start_x >= max_x as i32 {
        return;
    }
    let cw = fonts.char_width;
    if cw == 0 {
        return;
    }
    let avail_w = (max_x as i32 - start_x).max(0) as usize;
    let max_chars = avail_w / cw;
    let char_count = text.chars().count();

    if char_count <= max_chars {
        draw_string(
            fonts, frame, text, start_x, start_y, screen_w, screen_h, color,
        );
    } else if max_chars > 3 {
        let mut s: String = text.chars().take(max_chars - 3).collect();
        s.push_str("...");
        draw_string(
            fonts, frame, &s, start_x, start_y, screen_w, screen_h, color,
        );
    } else if max_chars > 0 {
        let s: String = text.chars().take(max_chars).collect();
        draw_string(
            fonts, frame, &s, start_x, start_y, screen_w, screen_h, color,
        );
    }
}

#[inline(always)]
pub fn draw_close_icon_clipped(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: i32,
    y: i32,
    size: usize,
    clip_min_x: usize,
    clip_max_x: usize,
    color: u32,
) {
    if size == 0 {
        return;
    }
    let s = size as i32;
    let max_x = clip_max_x.min(screen_w);
    for i in 0..size {
        let py = y + i as i32;
        if py < 0 || py as usize >= screen_h {
            continue;
        }
        let row_offset = py as usize * screen_w;
        let k = (i as i32).min(s - 1 - i as i32);
        let p1 = x + k;
        let p2 = x + k + 1;
        let p3 = x + s - 1 - k;
        let p4 = x + s - 2 - k;

        for px in [p1, p2, p3, p4] {
            if px >= clip_min_x as i32 && (px as usize) < max_x {
                buf[row_offset + px as usize] = color;
            }
        }
    }
}
