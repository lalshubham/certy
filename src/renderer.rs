use crate::buffer::EditorBuffer;
use crate::config::*;
use crate::font::FontManager;
use crate::layout::{calc_thumb, compute_layout, ViewportLayout};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::window::Window;

pub struct Renderer {
    _ctx: Context<Arc<Window>>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    pub width: usize,
    pub height: usize,
    pub font_manager: FontManager,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let ctx = Context::new(window.clone()).expect("Failed to initialize softbuffer context");
        let mut surface = Surface::new(&ctx, window.clone()).expect("Failed to create surface");
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            surface
                .resize(wnz, hnz)
                .expect("Failed to set surface size");
        }
        Self {
            _ctx: ctx,
            surface,
            width: w as usize,
            height: h as usize,
            font_manager: FontManager::new(),
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w as usize;
        self.height = h as usize;
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            self.surface
                .resize(wnz, hnz)
                .expect("Failed to resize surface");
        }
    }

    pub fn layout(&self, total_lines: usize) -> ViewportLayout {
        compute_layout(
            self.width,
            self.height,
            self.font_manager.char_width,
            self.font_manager.line_height,
            total_lines,
        )
    }

    pub fn render(&mut self, buffer: &EditorBuffer) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        let total_lines = buffer.text().len_lines();
        let layout = self.layout(total_lines);
        let (cur_line, cur_col) = buffer.cursor_pos();
        let sel_range = buffer.selection_range();

        let vert_thumb = calc_thumb(
            total_lines,
            layout.visible_lines,
            buffer.scroll_line,
            layout.content_bottom,
        );
        let horiz_track_w = layout.content_right.saturating_sub(layout.bar_start_x);
        let horiz_thumb = calc_thumb(
            buffer.max_line_len,
            layout.visible_cols,
            buffer.scroll_col,
            horiz_track_w,
        );

        let mut frame = self
            .surface
            .buffer_mut()
            .expect("Failed to get frame buffer");
        frame.fill(COLOR_BACKGROUND);

        draw_solid_rect(
            &mut frame,
            self.width,
            self.height,
            0,
            0,
            layout.gutter_width,
            self.height,
            COLOR_GUTTER_BACKGROUND,
        );
        draw_solid_rect(
            &mut frame,
            self.width,
            self.height,
            layout.gutter_width,
            0,
            1,
            self.height,
            COLOR_GUTTER_SEPARATOR,
        );

        let digits = total_lines.to_string().len().max(3);
        let lh = self.font_manager.line_height;
        let cw = self.font_manager.char_width;

        for row in 0..=layout.visible_lines {
            let line_idx = buffer.scroll_line + row;
            if line_idx >= total_lines {
                break;
            }

            let y = TOP_PADDING + row * lh;
            if y + lh > layout.content_bottom {
                break;
            }

            let num_str = format!("{:>width$}", line_idx + 1, width = digits);
            let num_color = if line_idx == cur_line {
                COLOR_LINE_NUMBER_ACTIVE
            } else {
                COLOR_LINE_NUMBER_MUTED
            };
            let mut nx = GUTTER_PADDING;
            for ch in num_str.chars() {
                self.font_manager.draw_char(
                    &mut frame,
                    ch,
                    nx,
                    y,
                    self.width,
                    self.height,
                    num_color,
                );
                nx += cw;
            }

            let line = buffer.text().line(line_idx);
            let line_start_char = buffer.text().line_to_char(line_idx);

            for (col_idx, ch) in line.chars().enumerate() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
                if col_idx < buffer.scroll_col {
                    continue;
                }

                let text_x = layout.code_x + (col_idx - buffer.scroll_col) * cw;
                if text_x + cw > layout.content_right {
                    break;
                }

                let char_idx = line_start_char + col_idx;
                if let Some((start, end)) = sel_range {
                    if char_idx >= start && char_idx < end {
                        draw_solid_rect(
                            &mut frame,
                            self.width,
                            self.height,
                            text_x,
                            y,
                            cw,
                            lh,
                            COLOR_SELECTION,
                        );
                    }
                }

                self.font_manager.draw_char(
                    &mut frame,
                    ch,
                    text_x,
                    y,
                    self.width,
                    self.height,
                    COLOR_TEXT_DEFAULT,
                );
            }
        }

        if cur_line >= buffer.scroll_line
            && cur_line < buffer.scroll_line + layout.visible_lines
            && cur_col >= buffer.scroll_col
            && cur_col <= buffer.scroll_col + layout.visible_cols
        {
            let cx = layout.code_x + (cur_col - buffer.scroll_col) * cw;
            let cy = TOP_PADDING + (cur_line - buffer.scroll_line) * lh;
            let max_y = (cy + lh).min(layout.content_bottom).min(self.height);
            let max_x = (cx + 2).min(layout.content_right).min(self.width);
            for y in cy.min(self.height)..max_y {
                for x in cx.min(self.width)..max_x {
                    frame[y * self.width + x] = COLOR_CURSOR;
                }
            }
        }

        if let Some((ty, th)) = vert_thumb {
            draw_solid_rect(
                &mut frame,
                self.width,
                self.height,
                layout.content_right,
                0,
                SCROLLBAR_THICKNESS,
                layout.content_bottom,
                COLOR_SCROLLBAR_TRACK,
            );
            draw_solid_rect(
                &mut frame,
                self.width,
                self.height,
                layout.content_right,
                ty,
                SCROLLBAR_THICKNESS,
                th,
                COLOR_SCROLLBAR_THUMB,
            );
        }
        if let Some((tx_offset, tw)) = horiz_thumb {
            let tx = layout.bar_start_x + tx_offset;
            draw_solid_rect(
                &mut frame,
                self.width,
                self.height,
                layout.bar_start_x,
                layout.content_bottom,
                horiz_track_w,
                SCROLLBAR_THICKNESS,
                COLOR_SCROLLBAR_TRACK,
            );
            draw_solid_rect(
                &mut frame,
                self.width,
                self.height,
                tx,
                layout.content_bottom,
                tw,
                SCROLLBAR_THICKNESS,
                COLOR_SCROLLBAR_THUMB,
            );
        }
        draw_solid_rect(
            &mut frame,
            self.width,
            self.height,
            layout.content_right,
            layout.content_bottom,
            SCROLLBAR_THICKNESS,
            SCROLLBAR_THICKNESS,
            COLOR_SCROLLBAR_TRACK,
        );

        frame.present().expect("Failed to present frame");
    }
}

#[inline(always)]
fn draw_solid_rect(
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
