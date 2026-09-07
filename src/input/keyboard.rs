use super::InputHandler;
use crate::editor::TabManager;
use crate::terminal::Terminal;
use crate::ui::layout::ViewportLayout;
use arboard::Clipboard;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

impl InputHandler {
    pub fn handle_key(
        &mut self,
        event: &KeyEvent,
        tabs: &mut TabManager,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        clipboard: &mut Option<Clipboard>,
    ) -> bool {
        if event.state == ElementState::Released {
            match &event.logical_key {
                Key::Named(NamedKey::Control) => self.ctrl_down = false,
                Key::Named(NamedKey::Shift) => self.shift_down = false,
                _ => {}
            }
            if matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight)
            ) {
                self.ctrl_down = false;
            }
            if matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight)
            ) {
                self.shift_down = false;
            }
            return false;
        }

        if self.context_menu.is_some() {
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
            {
                self.context_menu = None;
                return true;
            }
        }

        if tabs.closing_app || tabs.closing_files || tabs.pending_close.is_some() {
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
            {
                tabs.closing_app = false;
                tabs.closing_files = false;
                tabs.pending_close = None;
                return true;
            }
            return false;
        }

        if event.logical_key == Key::Named(NamedKey::Control)
            || matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight)
            )
        {
            self.ctrl_down = true;
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Shift)
            || matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight)
            )
        {
            self.shift_down = true;
            return false;
        }

        let is_ctrl = self.modifiers.control_key() || self.ctrl_down;
        let is_shift = self.modifiers.shift_key() || self.shift_down;

        if terminal.is_open && terminal.focused {
            let vis_rows = terminal.vis_rows(line_h);
            let text_left = layout.content_left + 14;
            let text_right = screen_w.saturating_sub(crate::config::SCROLLBAR_THICKNESS);
            let vis_cols = if char_w > 0 {
                text_right.saturating_sub(text_left) / char_w
            } else {
                0
            };

            let is_c = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyC))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("c") || c == "\u{3}",
                    _ => false,
                };
            let is_v = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyV))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("v") || c == "\u{16}",
                    _ => false,
                };

            let mut should_exit = false;

            if let Some(tab) = terminal.active_tab_mut() {
                if is_ctrl && is_c {
                    if let Some(text) = tab.selected_text() {
                        if let Some(cb) = clipboard.as_mut() {
                            let _ = cb.set_text(text);
                        }
                        return true;
                    }
                    tab.interrupt(vis_rows);
                    return true;
                }

                if is_ctrl && is_v {
                    if let Some(cb) = clipboard.as_mut() {
                        if let Ok(text) = cb.get_text() {
                            for ch in text.chars() {
                                if ch != '\n' && ch != '\r' {
                                    tab.insert_char(ch);
                                }
                            }
                            tab.selection_anchor = None;
                            tab.selection_end = None;
                            tab.ensure_cursor_visible(vis_cols);
                            return true;
                        }
                    }
                    return false;
                }

                if is_ctrl && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyL)) {
                    tab.lines.clear();
                    tab.partial_line.clear();
                    tab.scroll_line = 0;
                    tab.scroll_col = 0;
                    tab.selection_anchor = None;
                    tab.selection_end = None;
                    return true;
                }

                tab.selection_anchor = None;
                tab.selection_end = None;

                match &event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        tab.tab_complete(vis_rows);
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        if tab.execute_command(vis_rows) {
                            should_exit = true;
                        } else {
                            return true;
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        tab.delete_backwards();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Delete) => {
                        tab.delete_forward();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        tab.move_left();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        tab.move_right();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Home) => {
                        tab.cursor_col = 0;
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::End) => {
                        tab.cursor_col = tab.current_input.chars().count();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        tab.history_up();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        tab.history_down();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    _ => {
                        if !is_ctrl {
                            if let Some(txt) = &event.text {
                                for ch in txt.chars() {
                                    if !ch.is_control() {
                                        tab.insert_char(ch);
                                    }
                                }
                                tab.ensure_cursor_visible(vis_cols);
                                return true;
                            }
                        }
                    }
                }
            }

            if should_exit {
                let new_btn_w = "New".len() * char_w + 20;
                let strip_min_x = layout.content_left + new_btn_w;
                let strip_max_x = screen_w;
                let available_w = strip_max_x.saturating_sub(strip_min_x);
                let active_idx = terminal.active_idx;
                terminal.remove_terminal(active_idx, char_w, available_w);
                return true;
            }

            return false;
        }

        if let Some(tab) = tabs.active_tab_mut() {
            let buffer = &mut tab.buffer;

            let is_s = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("s") || c == "\u{13}",
                    _ => false,
                };
            let is_c = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyC))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("c") || c == "\u{3}",
                    _ => false,
                };
            let is_x = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyX))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("x") || c == "\u{18}",
                    _ => false,
                };
            let is_v = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyV))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("v") || c == "\u{16}",
                    _ => false,
                };
            let is_z = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyZ))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("z") || c == "\u{1a}",
                    _ => false,
                };
            let is_y = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyY))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("y") || c == "\u{19}",
                    _ => false,
                };
            let is_a = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyA))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("a") || c == "\u{1}",
                    _ => false,
                };

            if is_ctrl && is_s {
                let _ = buffer.save();
                if let Some(p) = &buffer.file_path {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        tab.title = name.to_string();
                    }
                }
                return true;
            }

            if is_ctrl && is_c {
                if let Some(text) = buffer.selected_text() {
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(text);
                    }
                }
                return false;
            }

            if is_ctrl && is_x {
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

            if is_ctrl && is_v {
                if let Some(cb) = clipboard.as_mut() {
                    if let Ok(text) = cb.get_text() {
                        buffer.insert_str(&text);
                        buffer.fit_view(layout.visible_lines, layout.visible_cols);
                        return true;
                    }
                }
                return false;
            }

            if is_ctrl && is_z {
                if is_shift {
                    buffer.redo();
                } else {
                    buffer.undo();
                }
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }

            if is_ctrl && is_y {
                buffer.redo();
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }

            if is_ctrl && is_a {
                buffer.select_all();
                return true;
            }

            match &event.logical_key {
                Key::Named(NamedKey::Backspace) => buffer.delete_backwards(),
                Key::Named(NamedKey::Delete) => buffer.delete_forward(),
                Key::Named(NamedKey::Enter) => buffer.insert_char('\n'),
                Key::Named(NamedKey::Tab) => {
                    if !is_ctrl {
                        let (_, col) = buffer.cursor_pos();
                        let spaces = 4 - (col % 4);
                        buffer.insert_str(&"    "[..spaces]);
                    }
                }
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
            return true;
        }

        false
    }
}
