use crate::terminal::Terminal;
use arboard::Clipboard;
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

pub fn handle_terminal_key(
    event: &KeyEvent,
    terminal: &mut Terminal,
    is_ctrl: bool,
    clipboard: &mut Option<Clipboard>,
) -> bool {
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
    false
}
