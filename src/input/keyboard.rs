use super::InputHandler;
use crate::editor::find::FindField;
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

        if tabs.find.is_open {
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
            {
                tabs.find.close();
                return true;
            }
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
        let is_alt = self.modifiers.alt_key();

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

        let active_idx = match tabs.active_idx {
            Some(i) => i,
            None => return false,
        };
        let (tab, find) = match tabs.tabs.get_mut(active_idx) {
            Some(t) => (t, &mut tabs.find),
            None => return false,
        };

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
        let is_f = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyF))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("f") || c == "\u{6}",
                _ => false,
            };
        let is_d = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyD))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("d") || c == "\u{4}",
                _ => false,
            };
        let is_k = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyK))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("k") || c == "\u{b}",
                _ => false,
            };
        let is_slash = matches!(
            event.physical_key,
            PhysicalKey::Code(KeyCode::Slash | KeyCode::NumpadDivide)
        ) || match &event.logical_key {
            Key::Character(c) => c == "/" || c == "\u{1f}",
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

        if is_ctrl && !is_shift && !is_alt && is_f {
            let sel = buffer.selected_text();
            find.open(find.is_replace, sel, buffer);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
            return true;
        }

        if is_ctrl && is_z {
            if is_shift {
                buffer.redo();
            } else {
                buffer.undo();
            }
            if find.is_open {
                find.update_matches(buffer);
                find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
            }
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        if is_ctrl && is_y {
            buffer.redo();
            if find.is_open {
                find.update_matches(buffer);
                find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
            }
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        if is_ctrl && is_a {
            if !find.is_open || !find.focused {
                buffer.select_all();
            } else {
                find.select_all();
            }
            return true;
        }

        if is_ctrl && !is_shift && !is_alt && is_d {
            buffer.duplicate_line();
            if find.is_open {
                find.update_matches(buffer);
            }
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        if is_ctrl && is_shift && !is_alt && is_k {
            buffer.delete_line();
            if find.is_open {
                find.update_matches(buffer);
            }
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        if is_ctrl && !is_shift && !is_alt && is_slash {
            buffer.toggle_line_comment();
            if find.is_open {
                find.update_matches(buffer);
            }
            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        if find.is_open && find.focused {
            if is_ctrl && is_c {
                if let Some(text) = find.selected_text() {
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(text);
                    }
                }
                return true;
            }
            if is_ctrl && is_x {
                if let Some(text) = find.selected_text() {
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(text);
                    }
                    find.delete_selection();
                    if find.active_field == FindField::Find {
                        find.update_matches(buffer);
                    }
                    find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                }
                return true;
            }
            if is_ctrl && is_v {
                if let Some(cb) = clipboard.as_mut() {
                    if let Ok(text) = cb.get_text() {
                        find.insert_str_at_cursor(&text, buffer);
                        find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                        return true;
                    }
                }
                return false;
            }

            let has_matches = !find.matches.is_empty();

            match &event.logical_key {
                Key::Named(NamedKey::Enter) => {
                    if find.active_field == crate::editor::find::FindField::Find {
                        if has_matches {
                            if is_shift {
                                find.prev_match(buffer, layout.visible_lines, layout.visible_cols);
                            } else {
                                find.next_match(buffer, layout.visible_lines, layout.visible_cols);
                            }
                        }
                    } else if has_matches {
                        find.replace_current(buffer, layout.visible_lines, layout.visible_cols);
                    }
                    return true;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    find.move_cursor_left();
                    return true;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    find.move_cursor_right();
                    return true;
                }
                Key::Named(NamedKey::Home) => {
                    find.move_cursor_home();
                    return true;
                }
                Key::Named(NamedKey::End) => {
                    find.move_cursor_end();
                    return true;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if has_matches {
                        find.next_match(buffer, layout.visible_lines, layout.visible_cols);
                    }
                    return true;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if has_matches {
                        find.prev_match(buffer, layout.visible_lines, layout.visible_cols);
                    }
                    return true;
                }
                Key::Named(NamedKey::Backspace) => {
                    find.delete_backwards(buffer);
                    find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                    return true;
                }
                Key::Named(NamedKey::Delete) => {
                    find.delete_forward(buffer);
                    find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                    return true;
                }
                _ => {
                    if !is_ctrl && !is_alt {
                        if let Some(txt) = &event.text {
                            for ch in txt.chars() {
                                if !ch.is_control() {
                                    find.insert_char_at_cursor(ch, buffer);
                                }
                            }
                            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                            return true;
                        }
                    }
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
                if find.is_open {
                    find.update_matches(buffer);
                }
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }
            return false;
        }

        if is_ctrl && is_v {
            if let Some(cb) = clipboard.as_mut() {
                if let Ok(text) = cb.get_text() {
                    buffer.insert_str(&text);
                    if find.is_open {
                        find.update_matches(buffer);
                    }
                    buffer.fit_view(layout.visible_lines, layout.visible_cols);
                    return true;
                }
            }
            return false;
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
            Key::Named(NamedKey::ArrowUp) => {
                if is_alt && !is_ctrl {
                    buffer.move_line_up();
                } else {
                    buffer.move_up(is_shift);
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if is_alt && !is_ctrl {
                    buffer.move_line_down();
                } else {
                    buffer.move_down(is_shift);
                }
            }
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

        if find.is_open {
            find.update_matches(buffer);
        }

        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        true
    }
}
