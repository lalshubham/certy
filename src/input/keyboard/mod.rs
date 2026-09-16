pub mod editor_keys;
pub mod overlay_keys;
pub mod terminal_keys;

use super::InputHandler;
use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use crate::ui::layout::ViewportLayout;
use arboard::Clipboard;
use editor_keys::{handle_editor_shortcut, handle_editor_text_input};
use overlay_keys::{handle_find_focused_key, handle_quick_open_key};
use terminal_keys::handle_terminal_key;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

impl InputHandler {
    pub fn handle_key(
        &mut self,
        event: &KeyEvent,
        tabs: &mut TabManager,
        sidebar: &Sidebar,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
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

        if self.context_menu.is_some()
            && event.state == ElementState::Pressed
            && matches!(event.logical_key, Key::Named(NamedKey::Escape))
        {
            self.context_menu = None;
            return true;
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

        if tabs.quick_open.is_open
            && event.state == ElementState::Pressed
            && matches!(event.logical_key, Key::Named(NamedKey::Escape))
        {
            tabs.quick_open.close();
            if tabs.is_find_visible() {
                tabs.find.focused = true;
            } else {
                tabs.focused = true;
            }
            return true;
        }

        if tabs.is_find_visible()
            && event.state == ElementState::Pressed
            && matches!(event.logical_key, Key::Named(NamedKey::Escape))
        {
            tabs.find.focused = false;
            tabs.focused = true;
            return true;
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
            return handle_terminal_key(event, terminal, is_ctrl, clipboard);
        }

        let is_p = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyP))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("p") || c == "\u{10}",
                _ => false,
            };

        if is_ctrl && !is_shift && !is_alt && is_p {
            if tabs.quick_open.is_open {
                tabs.quick_open.close();
                if tabs.is_find_visible() {
                    tabs.find.focused = true;
                } else {
                    tabs.focused = true;
                }
            } else {
                tabs.quick_open.open(sidebar.root_folder.as_deref());
                tabs.focused = false;
                terminal.focused = false;
                if tabs.is_find_visible() {
                    tabs.quick_open_above_find = true;
                    tabs.find.focused = false;
                }
            }
            return true;
        }

        if tabs.quick_open.is_open && tabs.quick_open.focused {
            let max_vis_chars = if char_w > 0 {
                layout
                    .content_right
                    .saturating_sub(layout.content_left + 100)
                    / char_w
            } else {
                20
            };
            if let Some(opened) = handle_quick_open_key(
                event,
                &mut tabs.quick_open,
                max_vis_chars,
                is_ctrl,
                is_shift,
                is_alt,
                clipboard,
            ) {
                if let Some(path) = opened {
                    tabs.open_file(path);
                    tabs.focused = true;
                }
                return true;
            }
        }

        let active_idx = match tabs.active_idx {
            Some(i) => i,
            None => return false,
        };

        let is_find_vis = tabs.is_find_visible();

        let (tab, find) = match tabs.tabs.get_mut(active_idx) {
            Some(t) => (t, &mut tabs.find),
            None => return false,
        };

        let max_vis_chars = if char_w > 0 {
            240usize.saturating_sub(8) / char_w
        } else {
            10
        };

        let is_f = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyF))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("f") || c == "\u{6}",
                _ => false,
            };

        let is_s = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
            || match &event.logical_key {
                Key::Character(c) => c.eq_ignore_ascii_case("s") || c == "\u{13}",
                _ => false,
            };

        if (is_ctrl && is_f) || (is_ctrl && is_s) || tabs.focused {
            if let Some(handled) = handle_editor_shortcut(
                event,
                tab,
                find,
                layout,
                max_vis_chars,
                is_ctrl,
                is_shift,
                is_alt,
            ) {
                if is_ctrl && is_f {
                    tabs.focused = false;
                }
                if tabs.quick_open.is_open && is_find_vis {
                    tabs.quick_open_above_find = false;
                    tabs.quick_open.focused = false;
                }
                return handled;
            }
        }

        if is_find_vis && find.focused {
            return handle_find_focused_key(
                event,
                find,
                &mut tab.buffer,
                layout,
                max_vis_chars,
                is_ctrl,
                is_shift,
                is_alt,
                clipboard,
            );
        }

        if tabs.focused {
            handle_editor_text_input(
                event,
                &mut tab.buffer,
                find,
                layout,
                is_ctrl,
                is_shift,
                is_alt,
                clipboard,
            )
        } else {
            false
        }
    }
}
