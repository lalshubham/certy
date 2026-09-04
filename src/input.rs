use crate::buffer::EditorBuffer;
use crate::config::{TOP_PADDING, TOUCHPAD_SCROLL_SPEED, WHEEL_SCROLL_SPEED};
use crate::layout::{calc_thumb, ViewportLayout};
use arboard::Clipboard;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key, ModifiersState, NamedKey};
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
    SelectingText,
}

#[derive(Default)]
pub struct InputHandler {
    pub drag: DragState,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub scroll_accum_y: f64,
    pub scroll_accum_x: f64,
    pub modifiers: ModifiersState,
}

impl InputHandler {
    pub fn desired_cursor_icon(&self, layout: &ViewportLayout) -> CursorIcon {
        if self.drag == DragState::SelectingText {
            return CursorIcon::Text;
        }
        if self.drag != DragState::None {
            return CursorIcon::Default;
        }
        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);
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
        char_w: usize,
        line_h: usize,
    ) -> bool {
        self.mouse_x = x;
        self.mouse_y = y;

        match self.drag {
            DragState::SelectingText => {
                let mx = self.mouse_x.max(0.0) as usize;
                let my = self.mouse_y.max(0.0) as usize;
                if line_h > 0 && char_w > 0 {
                    let row = my.saturating_sub(TOP_PADDING) / line_h;
                    let target_line = buffer.scroll_line + row;
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
            DragState::Vertical {
                start_y,
                start_line,
            } => {
                let total = buffer.text().len_lines();
                if let Some((_, th)) = calc_thumb(
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
                let track_w = layout.content_right.saturating_sub(layout.bar_start_x);
                if let Some((_, tw)) =
                    calc_thumb(max_c, layout.visible_cols, buffer.scroll_col, track_w)
                {
                    let travel = track_w.saturating_sub(tw) as f64;
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
            if self.drag == DragState::SelectingText
                && buffer.selection_anchor == Some(buffer.cursor_char)
            {
                buffer.selection_anchor = None;
            }
            self.drag = DragState::None;
            return false;
        }

        let total = buffer.text().len_lines();
        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);

        if mx >= layout.content_right && mx < screen_w && my < layout.content_bottom {
            if let Some((ty, th)) = calc_thumb(
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
            && mx >= layout.bar_start_x
            && mx < layout.content_right
        {
            let track_w = layout.content_right.saturating_sub(layout.bar_start_x);
            if let Some((tx_offset, tw)) = calc_thumb(
                buffer.max_line_len,
                layout.visible_cols,
                buffer.scroll_col,
                track_w,
            ) {
                let tx = layout.bar_start_x + tx_offset;
                if mx >= tx && mx < tx + tw {
                    self.drag = DragState::Horizontal {
                        start_x: self.mouse_x,
                        start_col: buffer.scroll_col,
                    };
                } else {
                    let ratio = ((mx - layout.bar_start_x) as f64 / track_w as f64).clamp(0.0, 1.0);
                    buffer.scroll_col =
                        (ratio * (buffer.max_line_len - layout.visible_cols) as f64) as usize;
                    self.drag = DragState::Horizontal {
                        start_x: self.mouse_x,
                        start_col: buffer.scroll_col,
                    };
                    return true;
                }
            }
        } else if my < layout.content_bottom
            && mx < layout.content_right
            && line_h > 0
            && char_w > 0
        {
            let row = my.saturating_sub(TOP_PADDING) / line_h;
            let target_line = buffer.scroll_line + row;
            let target_col = if mx >= layout.code_x {
                buffer.scroll_col + (mx - layout.code_x) / char_w
            } else {
                0
            };
            buffer.set_cursor_at(target_line, target_col);
            buffer.selection_anchor = Some(buffer.cursor_char);
            self.drag = DragState::SelectingText;
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
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
                buffer.scroll_line = (buffer.scroll_line as i32 - (v * WHEEL_SCROLL_SPEED) as i32)
                    .clamp(0, max_l as i32) as usize;
                buffer.scroll_col = (buffer.scroll_col as i32 - (h * WHEEL_SCROLL_SPEED) as i32)
                    .clamp(0, max_c as i32) as usize;
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_accum_y += pos.y * TOUCHPAD_SCROLL_SPEED;
                self.scroll_accum_x += pos.x * TOUCHPAD_SCROLL_SPEED;
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
        clipboard: &mut Option<Clipboard>,
    ) -> bool {
        if event.state != ElementState::Pressed {
            return false;
        }

        let is_ctrl = self.modifiers.control_key();
        let is_shift = self.modifiers.shift_key();

        if is_ctrl {
            match &event.logical_key {
                Key::Character(c) if c.eq_ignore_ascii_case("c") => {
                    if let Some(text) = buffer.selected_text() {
                        if let Some(cb) = clipboard.as_mut() {
                            let _ = cb.set_text(text);
                        }
                    }
                    return false;
                }
                Key::Character(c) if c.eq_ignore_ascii_case("x") => {
                    if let Some(text) = buffer.selected_text() {
                        if let Some(cb) = clipboard.as_mut() {
                            let _ = cb.set_text(text);
                        }
                        buffer.delete_selection();
                        buffer.fit_view(layout.visible_lines, layout.visible_cols);
                        return true;
                    }
                    return false;
                }
                Key::Character(c) if c.eq_ignore_ascii_case("v") => {
                    if let Some(cb) = clipboard.as_mut() {
                        if let Ok(text) = cb.get_text() {
                            buffer.insert_str(&text);
                            buffer.fit_view(layout.visible_lines, layout.visible_cols);
                            return true;
                        }
                    }
                    return false;
                }
                Key::Character(c) if c.eq_ignore_ascii_case("z") => {
                    if is_shift {
                        buffer.redo();
                    } else {
                        buffer.undo();
                    }
                    buffer.fit_view(layout.visible_lines, layout.visible_cols);
                    return true;
                }
                Key::Character(c) if c.eq_ignore_ascii_case("y") => {
                    buffer.redo();
                    buffer.fit_view(layout.visible_lines, layout.visible_cols);
                    return true;
                }
                Key::Character(c) if c.eq_ignore_ascii_case("a") => {
                    buffer.select_all();
                    return true;
                }
                _ => {}
            }
        }

        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => buffer.delete_backwards(),
            Key::Named(NamedKey::Delete) => buffer.delete_forward(),
            Key::Named(NamedKey::Enter) => buffer.insert_char('\n'),
            Key::Named(NamedKey::ArrowLeft) => buffer.move_left(is_shift),
            Key::Named(NamedKey::ArrowRight) => buffer.move_right(is_shift),
            Key::Named(NamedKey::ArrowUp) => buffer.move_up(is_shift),
            Key::Named(NamedKey::ArrowDown) => buffer.move_down(is_shift),
            _ => {
                if !is_ctrl {
                    if let Some(txt) = &event.text {
                        for ch in txt.chars() {
                            if !ch.is_control() {
                                buffer.insert_char(ch);
                            }
                        }
                    }
                }
            }
        }

        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        true
    }
}
