mod buffer;
mod config;
mod font;
mod history;
mod input;
mod layout;
mod renderer;
mod sidebar;
mod tabs;

use config::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
use input::{ActionEvent, InputHandler};
use renderer::Renderer;
use sidebar::{MenuItem, Sidebar};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tabs::TabManager;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, NamedKey, PhysicalKey},
    window::{CursorIcon, Window, WindowId},
};

#[derive(Debug)]
pub enum AppEvent {
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    SaveNewFile(PathBuf),
    CreateFolder(PathBuf),
}

fn to_full_path(path: &Path) -> String {
    if let Ok(canon) = fs::canonicalize(path) {
        canon.display().to_string()
    } else if path.is_absolute() {
        path.display().to_string()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path).display().to_string()
    } else {
        path.display().to_string()
    }
}

fn update_window_title(
    window: &Window,
    tabs: &TabManager,
    sidebar: &Sidebar,
    last_title: &mut String,
) {
    let title = if let Some(tab) = tabs.active_tab() {
        let path_str = if let Some(p) = &tab.buffer.file_path {
            to_full_path(p)
        } else {
            tab.title.clone()
        };
        format!("{path_str} - Certy")
    } else if let Some(root) = &sidebar.root_folder {
        format!("{} - Certy", to_full_path(root))
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        format!("{} - Certy", to_full_path(&cwd))
    };

    if *last_title != title {
        window.set_title(&title);
        *last_title = title;
    }
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: TabManager,
    sidebar: Sidebar,
    input: InputHandler,
    clipboard: Option<arboard::Clipboard>,
    active_cursor_icon: CursorIcon,
    current_title: String,
    event_proxy: EventLoopProxy<AppEvent>,
}

impl App {
    fn trigger_app_close(&mut self, event_loop: &ActiveEventLoop) {
        if self.tabs.has_modified() {
            self.tabs.closing_app = true;
            self.tabs.pending_close = None;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            event_loop.exit();
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attributes = Window::default_attributes()
                .with_title("Certy")
                .with_inner_size(LogicalSize::new(1024.0, 768.0))
                .with_min_inner_size(LogicalSize::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT));
            let window = Arc::new(
                event_loop
                    .create_window(attributes)
                    .expect("Failed to create window"),
            );
            let renderer = Renderer::new(window.clone());
            update_window_title(&window, &self.tabs, &self.sidebar, &mut self.current_title);
            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::OpenFile(path) => {
                self.tabs.open_file(path);
                if let Some(ref r) = self.renderer {
                    let avail_w = r.width.saturating_sub(self.sidebar.width);
                    self.tabs
                        .ensure_active_tab_visible(r.font_manager.char_width, avail_w);
                }
            }
            AppEvent::OpenFolder(path) => {
                self.sidebar.open_folder(path);
            }
            AppEvent::SaveNewFile(path) => {
                let _ = fs::File::create(&path);
                if let Some(ref root) = self.sidebar.root_folder {
                    if path.starts_with(root) {
                        self.sidebar.refresh_folder();
                    }
                }
                self.tabs.open_file(path);
                if let Some(ref r) = self.renderer {
                    let avail_w = r.width.saturating_sub(self.sidebar.width);
                    self.tabs
                        .ensure_active_tab_visible(r.font_manager.char_width, avail_w);
                }
            }
            AppEvent::CreateFolder(path) => {
                let _ = fs::create_dir_all(&path);
                if let Some(ref root) = self.sidebar.root_folder {
                    if path.starts_with(root) && &path != root {
                        self.sidebar.refresh_folder();
                    } else {
                        self.sidebar.open_folder(path);
                    }
                } else {
                    self.sidebar.open_folder(path);
                }
            }
        }
        if let Some(window) = &self.window {
            update_window_title(window, &self.tabs, &self.sidebar, &mut self.current_title);
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let (window, renderer) = match (&self.window, &mut self.renderer) {
            (Some(w), Some(r)) if w.id() == window_id => (w, r),
            _ => return,
        };

        let total_lines = self
            .tabs
            .active_tab()
            .map(|t| t.buffer.text().len_lines())
            .unwrap_or(0);
        let layout = renderer.layout(total_lines, self.sidebar.width);
        let cw = renderer.font_manager.char_width;
        let lh = renderer.font_manager.line_height;

        match event {
            WindowEvent::RedrawRequested => renderer.render(&self.tabs, &self.sidebar),

            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
                let avail_w = renderer.width.saturating_sub(self.sidebar.width);
                self.tabs.clamp_scroll(cw, avail_w);
                if let Some(tab) = self.tabs.active_tab_mut() {
                    let l = renderer.layout(tab.buffer.text().len_lines(), self.sidebar.width);
                    tab.buffer.fit_view(l.visible_lines, l.visible_cols);
                }
                window.request_redraw();
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.input.modifiers = modifiers.state();
            }

            WindowEvent::CursorMoved { position, .. } => {
                if self.input.handle_cursor_move(
                    position.x,
                    position.y,
                    &mut self.tabs,
                    &mut self.sidebar,
                    &layout,
                    cw,
                    lh,
                    renderer.width,
                    renderer.height,
                ) {
                    window.request_redraw();
                }

                let current_layout = renderer.layout(total_lines, self.sidebar.width);
                let desired_icon =
                    self.input
                        .desired_cursor_icon(&current_layout, &self.tabs, &self.sidebar);
                if self.active_cursor_icon != desired_icon {
                    self.active_cursor_icon = desired_icon;
                    window.set_cursor(desired_icon);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                match self.input.handle_mouse_click(
                    state,
                    button,
                    &mut self.tabs,
                    &mut self.sidebar,
                    &layout,
                    cw,
                    lh,
                    renderer.width,
                    renderer.height,
                ) {
                    ActionEvent::Menu(item) => match item {
                        MenuItem::NewFile => {
                            let proxy = self.event_proxy.clone();
                            let root_opt = self.sidebar.root_folder.clone();
                            std::thread::spawn(move || {
                                let mut dialog = rfd::FileDialog::new().set_title("New File");
                                if let Some(root) = root_opt {
                                    dialog = dialog.set_directory(&root);
                                }
                                if let Some(path) = dialog.save_file() {
                                    let _ = proxy.send_event(AppEvent::SaveNewFile(path));
                                }
                            });
                        }
                        MenuItem::NewFolder => {
                            let proxy = self.event_proxy.clone();
                            let root_opt = self.sidebar.root_folder.clone();
                            std::thread::spawn(move || {
                                let mut dialog = rfd::FileDialog::new().set_title("New Folder");
                                if let Some(root) = root_opt {
                                    dialog = dialog.set_directory(&root);
                                }
                                if let Some(path) = dialog.save_file() {
                                    let _ = proxy.send_event(AppEvent::CreateFolder(path));
                                }
                            });
                        }
                        MenuItem::OpenFile => {
                            let proxy = self.event_proxy.clone();
                            std::thread::spawn(move || {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    let _ = proxy.send_event(AppEvent::OpenFile(path));
                                }
                            });
                        }
                        MenuItem::OpenFolder => {
                            let proxy = self.event_proxy.clone();
                            std::thread::spawn(move || {
                                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                    let _ = proxy.send_event(AppEvent::OpenFolder(dir));
                                }
                            });
                        }
                        MenuItem::Save => {
                            if let Some(tab) = self.tabs.active_tab_mut() {
                                let _ = tab.buffer.save();
                                if let Some(p) = &tab.buffer.file_path {
                                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                        tab.title = name.to_string();
                                    }
                                }
                                if self.sidebar.root_folder.is_some() {
                                    self.sidebar.refresh_folder();
                                }
                                update_window_title(
                                    window,
                                    &self.tabs,
                                    &self.sidebar,
                                    &mut self.current_title,
                                );
                                window.request_redraw();
                            }
                        }
                    },
                    ActionEvent::OpenFile(path) => {
                        self.tabs.open_file(path);
                        let avail_w = renderer.width.saturating_sub(self.sidebar.width);
                        self.tabs.ensure_active_tab_visible(cw, avail_w);
                        update_window_title(
                            window,
                            &self.tabs,
                            &self.sidebar,
                            &mut self.current_title,
                        );
                        window.request_redraw();
                    }
                    ActionEvent::SaveTab(idx) => {
                        if let Some(tab) = self.tabs.tabs.get_mut(idx) {
                            let _ = tab.buffer.save();
                        }
                        if self.sidebar.root_folder.is_some() {
                            self.sidebar.refresh_folder();
                        }
                        self.tabs.close_tab(idx);
                        let avail_w = renderer.width.saturating_sub(self.sidebar.width);
                        self.tabs.clamp_scroll(cw, avail_w);
                        self.tabs.ensure_active_tab_visible(cw, avail_w);
                        self.input.handle_cursor_move(
                            self.input.mouse_x,
                            self.input.mouse_y,
                            &mut self.tabs,
                            &mut self.sidebar,
                            &layout,
                            cw,
                            lh,
                            renderer.width,
                            renderer.height,
                        );
                        update_window_title(
                            window,
                            &self.tabs,
                            &self.sidebar,
                            &mut self.current_title,
                        );
                        window.request_redraw();
                    }
                    ActionEvent::DiscardTab(idx) => {
                        self.tabs.close_tab(idx);
                        let avail_w = renderer.width.saturating_sub(self.sidebar.width);
                        self.tabs.clamp_scroll(cw, avail_w);
                        self.tabs.ensure_active_tab_visible(cw, avail_w);
                        self.input.handle_cursor_move(
                            self.input.mouse_x,
                            self.input.mouse_y,
                            &mut self.tabs,
                            &mut self.sidebar,
                            &layout,
                            cw,
                            lh,
                            renderer.width,
                            renderer.height,
                        );
                        update_window_title(
                            window,
                            &self.tabs,
                            &self.sidebar,
                            &mut self.current_title,
                        );
                        window.request_redraw();
                    }
                    ActionEvent::SaveAllAndExit => {
                        for tab in &mut self.tabs.tabs {
                            if tab.buffer.is_modified {
                                let _ = tab.buffer.save();
                            }
                        }
                        if self.sidebar.root_folder.is_some() {
                            self.sidebar.refresh_folder();
                        }
                        event_loop.exit();
                    }
                    ActionEvent::DiscardAllAndExit => {
                        event_loop.exit();
                    }
                    ActionEvent::CancelClose => {
                        self.tabs.pending_close = None;
                        self.tabs.closing_app = false;
                        self.input.handle_cursor_move(
                            self.input.mouse_x,
                            self.input.mouse_y,
                            &mut self.tabs,
                            &mut self.sidebar,
                            &layout,
                            cw,
                            lh,
                            renderer.width,
                            renderer.height,
                        );
                        update_window_title(
                            window,
                            &self.tabs,
                            &self.sidebar,
                            &mut self.current_title,
                        );
                        window.request_redraw();
                    }
                    ActionEvent::Redraw => {
                        update_window_title(
                            window,
                            &self.tabs,
                            &self.sidebar,
                            &mut self.current_title,
                        );
                        window.request_redraw();
                    }
                    ActionEvent::None => {}
                }

                let current_layout = renderer.layout(total_lines, self.sidebar.width);
                let desired_icon =
                    self.input
                        .desired_cursor_icon(&current_layout, &self.tabs, &self.sidebar);
                if self.active_cursor_icon != desired_icon {
                    self.active_cursor_icon = desired_icon;
                    window.set_cursor(desired_icon);
                }
            }

            WindowEvent::Focused(is_focused) => {
                if !is_focused {
                    self.input.drag = input::DragState::None;
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if self.input.handle_mouse_wheel(
                    delta,
                    &mut self.tabs,
                    &mut self.sidebar,
                    &layout,
                    cw,
                    lh,
                    renderer.width,
                    renderer.height,
                ) {
                    window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let is_alt = self.input.modifiers.alt_key();
                let is_f4 = matches!(event.physical_key, PhysicalKey::Code(KeyCode::F4))
                    || matches!(event.logical_key, winit::keyboard::Key::Named(NamedKey::F4));
                if event.state == ElementState::Pressed && is_alt && is_f4 {
                    self.trigger_app_close(event_loop);
                    return;
                }

                if self
                    .input
                    .handle_key(&event, &mut self.tabs, &layout, &mut self.clipboard)
                {
                    if self.sidebar.root_folder.is_some() {
                        self.sidebar.refresh_folder();
                    }
                    update_window_title(window, &self.tabs, &self.sidebar, &mut self.current_title);
                    window.request_redraw();
                }
            }

            WindowEvent::CloseRequested => {
                self.trigger_app_close(event_loop);
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let event_proxy = event_loop.create_proxy();

    let mut app = App {
        window: None,
        renderer: None,
        tabs: TabManager::new(),
        sidebar: Sidebar::new(),
        input: InputHandler::default(),
        clipboard: arboard::Clipboard::new().ok(),
        active_cursor_icon: CursorIcon::Default,
        current_title: String::new(),
        event_proxy,
    };

    event_loop
        .run_app(&mut app)
        .expect("Error running event loop");
}
