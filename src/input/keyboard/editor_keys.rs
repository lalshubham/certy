use crate::editor::buffer::EditorBuffer;
use crate::editor::find::FindState;
use crate::editor::tabs::Tab;
use crate::ui::layout::ViewportLayout;
use arboard::Clipboard;
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

pub fn handle_editor_shortcut(
    event: &KeyEvent,
    tab: &mut Tab,
    find: &mut FindState,
    layout: &ViewportLayout,
    max_vis_chars: usize,
    is_ctrl: bool,
    is_shift: bool,
    is_alt: bool,
) -> Option<bool> {
    let is_s = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
        || match &event.logical_key {
            Key::Character(c) => c.eq_ignore_ascii_case("s") || c == "\u{13}",
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

    let buffer = &mut tab.buffer;

    if is_ctrl && is_s {
        let _ = buffer.save();
        if let Some(p) = &buffer.file_path {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                tab.title = name.to_string();
            }
        }
        return Some(true);
    }

    if is_ctrl && !is_shift && !is_alt && is_f {
        if !find.is_open {
            let sel = buffer.selected_text();
            find.open(find.is_replace, sel, buffer);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
        } else {
            find.focused = true;
            if let Some(sel) = buffer.selected_text() {
                if !sel.is_empty() && !sel.contains('\n') {
                    find.query = sel;
                    find.query_cursor = find.query.chars().count();
                    find.query_selection_anchor = None;
                    find.update_matches(buffer);
                    find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                }
            }
        }
        return Some(true);
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
        return Some(true);
    }

    if is_ctrl && is_y {
        buffer.redo();
        if find.is_open {
            find.update_matches(buffer);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
        }
        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        return Some(true);
    }

    if is_ctrl && is_a {
        if !find.is_open || !find.focused {
            buffer.select_all();
        } else {
            find.select_all();
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
        }
        return Some(true);
    }

    if is_ctrl && !is_shift && !is_alt && is_d {
        buffer.duplicate_line();
        if find.is_open {
            find.update_matches(buffer);
        }
        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        return Some(true);
    }

    if is_ctrl && is_shift && !is_alt && is_k {
        buffer.delete_line();
        if find.is_open {
            find.update_matches(buffer);
        }
        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        return Some(true);
    }

    if is_ctrl && !is_shift && !is_alt && is_slash {
        buffer.toggle_line_comment();
        if find.is_open {
            find.update_matches(buffer);
        }
        buffer.fit_view(layout.visible_lines, layout.visible_cols);
        return Some(true);
    }

    None
}

pub fn handle_editor_text_input(
    event: &KeyEvent,
    buffer: &mut EditorBuffer,
    find: &mut FindState,
    layout: &ViewportLayout,
    is_ctrl: bool,
    is_shift: bool,
    is_alt: bool,
    clipboard: &mut Option<Clipboard>,
) -> bool {
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
        Key::Named(NamedKey::Home) => buffer.move_home(is_shift),
        Key::Named(NamedKey::End) => buffer.move_end(is_shift),
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
