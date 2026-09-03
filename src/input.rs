use crate::buffer::EditorBuffer;
use crate::config::TOP_PADDING;
use crate::layout::{calc_horiz_thumb, calc_vert_thumb, ViewportLayout};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key, NamedKey};
use winit::window::CursorIcon;

#[derive(Default, PartialEq)]
pub enum DragState {
    #[default]
    None,
    Vertical {
        start_y: f64,
        start_line: usize,
    },
    Horizontal {
        start_x: f64,
        start_col: usize,
    },
}

#[derive(Default)]
pub struct InputHandler {
    pub drag: DragState,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub scroll_accum_y: f64,
    pub scroll_accum_x: f64,
}

impl InputHandler {
    pub fn desired_cursor_icon(&self, layout: &ViewportLayout) -> CursorIcon {
        if self.drag != DragState::None {
            return CursorIcon::Default;
        }

        let mx = self.mouse_x as usize;
        let my = self.mouse_y as usize;

        if my < layout.content_bottom && mx >= layout.code_x && mx < layout.content_right {
            CursorIcon::Text
        } else {
            CursorIcon::Default
        }
    }

    pub fn handle_cursor_move(
        &mut self,
        x: f64,
        y: f64,
        buffer: &mut EditorBuffer,
        layout: &ViewportLayout,
    ) -> bool {
        self.mouse_x = x;
        self.mouse_y = y;

        match self.drag {
            DragState::Vertical {
                start_y,
                start_line,
            } => {
                let total = buffer.text().len_lines();
                if let Some((_, th)) = calc_vert_thumb(
                    total,
                    layout.visible_lines,
                    buffer.scroll_line,
                    layout.content_bottom,
                ) {
                    let travel = (layout.content_bottom.saturating_sub(th)) as f64;
                    if travel > 0.0 {
                        let max_s = (total - layout.visible_lines) as f64;
                        let target = (start_line as f64
                            + ((self.mouse_y - start_y) / travel) * max_s)
                            .clamp(0.0, max_s) as usize;
                        if buffer.scroll_line != target {
                            buffer.scroll_line = target;
                            return true;
                        }
                    }
                }
            }
            DragState::Horizontal { start_x, start_col } => {
                let max_c = buffer.max_line_len;
                let bar_start_x = layout.gutter_width + 1;
                if let Some((_, tw)) = calc_horiz_thumb(
                    max_c,
                    layout.visible_cols,
                    buffer.scroll_col,
                    bar_start_x,
                    layout.content_right,
                ) {
                    let travel = (layout.content_right.saturating_sub(bar_start_x + tw)) as f64;
                    if travel > 0.0 {
                        let max_s = (max_c - layout.visible_cols) as f64;
                        let target = (start_col as f64
                            + ((self.mouse_x - start_x) / travel) * max_s)
                            .clamp(0.0, max_s) as usize;
                        if buffer.scroll_col != target {
                            buffer.scroll_col = target;
                            return true;
                        }
                    }
                }
            }
            DragState::None => {}
        }
        false
    }

    pub fn handle_mouse_click(
        &mut self,
        state: ElementState,
        button: MouseButton,
        buffer: &mut EditorBuffer,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        if state == ElementState::Released {
            self.drag = DragState::None;
            return false;
        }

        let total = buffer.text().len_lines();
        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);
        let bar_start_x = layout.gutter_width + 1;

        if mx >= layout.content_right && mx < screen_w && my < layout.content_bottom {
            if let Some((ty, th)) = calc_vert_thumb(
                total,
                layout.visible_lines,
                buffer.scroll_line,
                layout.content_bottom,
            ) {
                if my >= ty && my < ty + th {
                    self.drag = DragState::Vertical {
                        start_y: self.mouse_y,
                        start_line: buffer.scroll_line,
                    };
                } else {
                    let ratio = (my as f64 / layout.content_bottom as f64).clamp(0.0, 1.0);
                    buffer.scroll_line = (ratio * (total - layout.visible_lines) as f64) as usize;
                    self.drag = DragState::Vertical {
                        start_y: self.mouse_y,
                        start_line: buffer.scroll_line,
                    };
                    return true;
                }
            }
        } else if my >= layout.content_bottom
            && my < screen_h
            && mx >= bar_start_x
            && mx < layout.content_right
        {
            if let Some((tx, tw)) = calc_horiz_thumb(
                buffer.max_line_len,
                layout.visible_cols,
                buffer.scroll_col,
                bar_start_x,
                layout.content_right,
            ) {
                if mx >= tx && mx < tx + tw {
                    self.drag = DragState::Horizontal {
                        start_x: self.mouse_x,
                        start_col: buffer.scroll_col,
                    };
                } else {
                    let track_w = (layout.content_right - bar_start_x) as f64;
                    let ratio = ((mx - bar_start_x) as f64 / track_w).clamp(0.0, 1.0);
                    buffer.scroll_col =
                        (ratio * (buffer.max_line_len - layout.visible_cols) as f64) as usize;
                    self.drag = DragState::Horizontal {
                        start_x: self.mouse_x,
                        start_col: buffer.scroll_col,
                    };
                    return true;
                }
            }
        } else {
            self.drag = DragState::None;
            if my < layout.content_bottom && mx < layout.content_right && line_h > 0 && char_w > 0 {
                let click_row = my.saturating_sub(TOP_PADDING) / line_h;
                let target_line = buffer.scroll_line + click_row;
                let target_col = if mx >= layout.code_x {
                    buffer.scroll_col + (mx - layout.code_x) / char_w
                } else {
                    0
                };
                buffer.set_cursor_at(target_line, target_col);
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }
        }
        false
    }

    pub fn handle_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        buffer: &mut EditorBuffer,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
    ) -> bool {
        match delta {
            MouseScrollDelta::LineDelta(h, v) => {
                let max_l = buffer
                    .text()
                    .len_lines()
                    .saturating_sub(layout.visible_lines);
                let max_c = buffer.max_line_len.saturating_sub(layout.visible_cols);
                buffer.scroll_line =
                    (buffer.scroll_line as i32 - (v * 3.0) as i32).clamp(0, max_l as i32) as usize;
                buffer.scroll_col =
                    (buffer.scroll_col as i32 - (h * 3.0) as i32).clamp(0, max_c as i32) as usize;
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_accum_y += pos.y;
                self.scroll_accum_x += pos.x;

                let lines = (self.scroll_accum_y / line_h as f64) as i32;
                let cols = (self.scroll_accum_x / char_w as f64) as i32;

                if lines != 0 {
                    let max_l = buffer
                        .text()
                        .len_lines()
                        .saturating_sub(layout.visible_lines);
                    buffer.scroll_line =
                        (buffer.scroll_line as i32 - lines).clamp(0, max_l as i32) as usize;
                    self.scroll_accum_y -= lines as f64 * line_h as f64;
                }
                if cols != 0 {
                    let max_c = buffer.max_line_len.saturating_sub(layout.visible_cols);
                    buffer.scroll_col =
                        (buffer.scroll_col as i32 - cols).clamp(0, max_c as i32) as usize;
                    self.scroll_accum_x -= cols as f64 * char_w as f64;
                }
            }
        }
        true
    }

    pub fn handle_key(
        &self,
        event: &KeyEvent,
        buffer: &mut EditorBuffer,
        layout: &ViewportLayout,
    ) -> bool {
        if event.state != ElementState::Pressed {
            return false;
        }

        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => buffer.delete_backwards(),
            Key::Named(NamedKey::Delete) => buffer.delete_forward(),
            Key::Named(NamedKey::Enter) => buffer.insert_char('\n'),
            Key::Named(NamedKey::ArrowLeft) => buffer.move_left(),
            Key::Named(NamedKey::ArrowRight) => buffer.move_right(),
            Key::Named(NamedKey::ArrowUp) => buffer.move_up(),
            Key::Named(NamedKey::ArrowDown) => buffer.move_down(),
            _ => {
                if let Some(txt) = &event.text {
                    for ch in txt.chars() {
                        if !ch.is_control() {
                            buffer.insert_char(ch);
                        }
                    }
                }
            }
        }

        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        true
    }
}
