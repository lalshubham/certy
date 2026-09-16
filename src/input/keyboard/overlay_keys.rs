use crate::editor::buffer::EditorBuffer;
use crate::editor::find::{FindField, FindState};
use crate::editor::quick_open::QuickOpenState;
use crate::ui::layout::ViewportLayout;
use arboard::Clipboard;
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

pub fn handle_quick_open_key(
    event: &KeyEvent,
    quick_open: &mut QuickOpenState,
    max_vis_chars: usize,
    is_ctrl: bool,
    is_shift: bool,
    is_alt: bool,
    clipboard: &mut Option<Clipboard>,
) -> Option<Option<std::path::PathBuf>> {
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
    let is_a = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyA))
        || match &event.logical_key {
            Key::Character(c) => c.eq_ignore_ascii_case("a") || c == "\u{1}",
            _ => false,
        };

    if is_ctrl && is_c {
        if let Some(text) = quick_open.selected_text() {
            if let Some(cb) = clipboard.as_mut() {
                let _ = cb.set_text(text);
            }
        }
        return Some(None);
    }
    if is_ctrl && is_x {
        if let Some(text) = quick_open.selected_text() {
            if let Some(cb) = clipboard.as_mut() {
                let _ = cb.set_text(text);
            }
            quick_open.delete_selection();
            quick_open.ensure_query_visible(max_vis_chars);
        }
        return Some(None);
    }
    if is_ctrl && is_v {
        if let Some(cb) = clipboard.as_mut() {
            if let Ok(text) = cb.get_text() {
                quick_open.insert_str_at_cursor(&text);
                quick_open.ensure_query_visible(max_vis_chars);
                return Some(None);
            }
        }
        return Some(None);
    }
    if is_ctrl && is_a {
        quick_open.select_all();
        quick_open.ensure_query_visible(max_vis_chars);
        return Some(None);
    }

    match &event.logical_key {
        Key::Named(NamedKey::Enter) => {
            let path = quick_open.selected_file();
            quick_open.close();
            Some(path)
        }
        Key::Named(NamedKey::ArrowUp) => {
            if is_shift {
                quick_open.move_cursor_up(true);
            } else {
                quick_open.prev_match();
            }
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::ArrowDown) => {
            if is_shift {
                quick_open.move_cursor_down(true);
            } else {
                quick_open.next_match();
            }
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::ArrowLeft) => {
            quick_open.move_cursor_left(is_shift);
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::ArrowRight) => {
            quick_open.move_cursor_right(is_shift);
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::Home) => {
            quick_open.move_cursor_home(is_shift);
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::End) => {
            quick_open.move_cursor_end(is_shift);
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::Backspace) => {
            quick_open.delete_backwards();
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        Key::Named(NamedKey::Delete) => {
            quick_open.delete_forward();
            quick_open.ensure_query_visible(max_vis_chars);
            Some(None)
        }
        _ => {
            if !is_ctrl && !is_alt {
                if let Some(txt) = &event.text {
                    for ch in txt.chars() {
                        if !ch.is_control() {
                            quick_open.insert_char_at_cursor(ch);
                        }
                    }
                    quick_open.ensure_query_visible(max_vis_chars);
                    return Some(None);
                }
            }
            Some(None)
        }
    }
}

pub fn handle_find_focused_key(
    event: &KeyEvent,
    find: &mut FindState,
    buffer: &mut EditorBuffer,
    layout: &ViewportLayout,
    max_vis_chars: usize,
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
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
        }
        return true;
    }
    if is_ctrl && is_v {
        if let Some(cb) = clipboard.as_mut() {
            if let Ok(text) = cb.get_text() {
                find.insert_str_at_cursor(&text, buffer);
                find.ensure_query_visible(max_vis_chars);
                find.ensure_replace_visible(max_vis_chars);
                find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                return true;
            }
        }
        return false;
    }

    let has_matches = !find.matches.is_empty();
    match &event.logical_key {
        Key::Named(NamedKey::Enter) => {
            if find.active_field == FindField::Find {
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
            true
        }
        Key::Named(NamedKey::ArrowLeft) => {
            find.move_cursor_left(is_shift);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::ArrowRight) => {
            find.move_cursor_right(is_shift);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::Home) => {
            find.move_cursor_home(is_shift);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::End) => {
            find.move_cursor_end(is_shift);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::ArrowDown) => {
            if is_shift {
                find.move_cursor_down(true);
            } else if has_matches {
                find.next_match(buffer, layout.visible_lines, layout.visible_cols);
            } else {
                find.move_cursor_down(false);
            }
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::ArrowUp) => {
            if is_shift {
                find.move_cursor_up(true);
            } else if has_matches {
                find.prev_match(buffer, layout.visible_lines, layout.visible_cols);
            } else {
                find.move_cursor_up(false);
            }
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            true
        }
        Key::Named(NamedKey::Backspace) => {
            find.delete_backwards(buffer);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
            true
        }
        Key::Named(NamedKey::Delete) => {
            find.delete_forward(buffer);
            find.ensure_query_visible(max_vis_chars);
            find.ensure_replace_visible(max_vis_chars);
            find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
            true
        }
        _ => {
            if !is_ctrl && !is_alt {
                if let Some(txt) = &event.text {
                    for ch in txt.chars() {
                        if !ch.is_control() {
                            find.insert_char_at_cursor(ch, buffer);
                        }
                    }
                    find.ensure_query_visible(max_vis_chars);
                    find.ensure_replace_visible(max_vis_chars);
                    find.sync_view(buffer, layout.visible_lines, layout.visible_cols);
                    return true;
                }
            }
            true
        }
    }
}
