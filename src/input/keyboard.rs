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
        _char_w: usize,
        _line_h: usize,
        _screen_w: usize,
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

            if let Some(tab) = terminal.active_tab_mut() {
                if is_ctrl && is_c {
                    if let Some(text) = tab.selected_text() {
                        if let Some(cb) = clipboard.as_mut() {
                            let _ = cb.set_text(text);
                        }
                        return true;
                    }
                    tab.write_bytes(b"\x03");
                    return true;
                }

                if is_ctrl && is_v {
                    if let Some(cb) = clipboard.as_mut() {
                        if let Ok(text) = cb.get_text() {
                            tab.write_bytes(text.as_bytes());
                            return true;
                        }
                    }
                    return false;
                }

                if is_ctrl {
                    let ctrl_byte = match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyA) => Some(b"\x01"),
                        PhysicalKey::Code(KeyCode::KeyB) => Some(b"\x02"),
                        PhysicalKey::Code(KeyCode::KeyD) => Some(b"\x04"),
                        PhysicalKey::Code(KeyCode::KeyE) => Some(b"\x05"),
                        PhysicalKey::Code(KeyCode::KeyK) => Some(b"\x0b"),
                        PhysicalKey::Code(KeyCode::KeyL) => Some(b"\x0c"),
                        PhysicalKey::Code(KeyCode::KeyN) => Some(b"\x0e"),
                        PhysicalKey::Code(KeyCode::KeyP) => Some(b"\x10"),
                        PhysicalKey::Code(KeyCode::KeyR) => Some(b"\x12"),
                        PhysicalKey::Code(KeyCode::KeyT) => Some(b"\x14"),
                        PhysicalKey::Code(KeyCode::KeyU) => Some(b"\x15"),
                        PhysicalKey::Code(KeyCode::KeyW) => Some(b"\x17"),
                        PhysicalKey::Code(KeyCode::KeyX) => Some(b"\x18"),
                        PhysicalKey::Code(KeyCode::KeyY) => Some(b"\x19"),
                        PhysicalKey::Code(KeyCode::KeyZ) => Some(b"\x1a"),
                        _ => None,
                    };
                    if let Some(b) = ctrl_byte {
                        tab.write_bytes(b);
                        return true;
                    }
                }

                let key_bytes: Option<&[u8]> = match &event.logical_key {
                    Key::Named(NamedKey::Enter) => Some(b"\r"),
                    Key::Named(NamedKey::Backspace) => Some(b"\x7f"),
                    Key::Named(NamedKey::Tab) => Some(b"\t"),
                    Key::Named(NamedKey::Escape) => Some(b"\x1b"),
                    Key::Named(NamedKey::ArrowUp) => Some(b"\x1b[A"),
                    Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B"),
                    Key::Named(NamedKey::ArrowRight) => Some(b"\x1b[C"),
                    Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D"),
                    Key::Named(NamedKey::Home) => Some(b"\x1b[H"),
                    Key::Named(NamedKey::End) => Some(b"\x1b[F"),
                    Key::Named(NamedKey::PageUp) => Some(b"\x1b[5~"),
                    Key::Named(NamedKey::PageDown) => Some(b"\x1b[6~"),
                    Key::Named(NamedKey::Delete) => Some(b"\x1b[3~"),
                    _ => None,
                };

                if let Some(b) = key_bytes {
                    tab.write_bytes(b);
                    return true;
                }

                if !is_ctrl {
                    if let Some(txt) = &event.text {
                        tab.write_bytes(txt.as_bytes());
                        return true;
                    }
                }
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
                Key::Named(NamedKey::Enter) => buffer.insert_newline(),
                Key::Named(NamedKey::Tab) => {
                    if !is_ctrl {
                        if is_shift {
                            buffer.unindent_selection();
                        } else if buffer.selection_range().is_some() {
                            buffer.indent_selection();
                        } else {
                            let (_, col) = buffer.cursor_pos();
                            let spaces = 4 - (col % 4);
                            buffer.insert_str(&"    "[..spaces]);
                        }
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
                                    buffer.type_char(ch);
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
