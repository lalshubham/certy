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
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{CursorIcon, Window, WindowBuilder},
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

fn main() {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");
    let event_proxy = event_loop.create_proxy();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Certy")
            .with_inner_size(LogicalSize::new(1024.0, 768.0))
            .with_min_inner_size(LogicalSize::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT))
            .build(&event_loop)
            .expect("Failed to create window"),
    );

    let mut renderer = Renderer::new(window.clone());
    let mut tabs = TabManager::new();
    let mut sidebar = Sidebar::new();
    let mut input = InputHandler::default();
    let mut clipboard = arboard::Clipboard::new().ok();
    let mut active_cursor_icon = CursorIcon::Default;
    let mut current_title = String::new();

    update_window_title(&window, &tabs, &sidebar, &mut current_title);
    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::UserEvent(app_event) => {
                    match app_event {
                        AppEvent::OpenFile(path) => {
                            tabs.open_file(path);
                        }
                        AppEvent::OpenFolder(path) => {
                            sidebar.open_folder(path);
                        }
                        AppEvent::SaveNewFile(path) => {
                            let _ = fs::File::create(&path);
                            if let Some(ref root) = sidebar.root_folder {
                                if path.starts_with(root) {
                                    sidebar.refresh_folder();
                                }
                            }
                            tabs.open_file(path);
                        }
                        AppEvent::CreateFolder(path) => {
                            let _ = fs::create_dir_all(&path);
                            if let Some(ref root) = sidebar.root_folder {
                                if path.starts_with(root) && &path != root {
                                    sidebar.refresh_folder();
                                } else {
                                    sidebar.open_folder(path);
                                }
                            } else {
                                sidebar.open_folder(path);
                            }
                        }
                    }
                    update_window_title(&window, &tabs, &sidebar, &mut current_title);
                    window.request_redraw();
                }

                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    let total_lines = tabs
                        .active_tab()
                        .map(|t| t.buffer.text().len_lines())
                        .unwrap_or(0);
                    let layout = renderer.layout(total_lines, sidebar.width);
                    let cw = renderer.font_manager.char_width;
                    let lh = renderer.font_manager.line_height;

                    match event {
                        WindowEvent::RedrawRequested => renderer.render(&tabs, &sidebar),

                        WindowEvent::Resized(size) => {
                            renderer.resize(size.width, size.height);
                            if let Some(tab) = tabs.active_tab_mut() {
                                let l =
                                    renderer.layout(tab.buffer.text().len_lines(), sidebar.width);
                                tab.buffer.fit_view(l.visible_lines, l.visible_cols);
                            }
                            window.request_redraw();
                        }

                        WindowEvent::ModifiersChanged(modifiers) => {
                            input.modifiers = modifiers.state();
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            if input.handle_cursor_move(
                                position.x,
                                position.y,
                                &mut tabs,
                                &mut sidebar,
                                &layout,
                                cw,
                                lh,
                                renderer.width,
                                renderer.height,
                            ) {
                                window.request_redraw();
                            }

                            let current_layout = renderer.layout(total_lines, sidebar.width);
                            let desired_icon =
                                input.desired_cursor_icon(&current_layout, &tabs, &sidebar);
                            if active_cursor_icon != desired_icon {
                                active_cursor_icon = desired_icon;
                                window.set_cursor_icon(desired_icon);
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            match input.handle_mouse_click(
                                state,
                                button,
                                &mut tabs,
                                &mut sidebar,
                                &layout,
                                cw,
                                lh,
                                renderer.width,
                                renderer.height,
                            ) {
                                ActionEvent::Menu(item) => match item {
                                    MenuItem::NewFile => {
                                        let proxy = event_proxy.clone();
                                        let root_opt = sidebar.root_folder.clone();
                                        std::thread::spawn(move || {
                                            let mut dialog =
                                                rfd::FileDialog::new().set_title("New File");
                                            if let Some(root) = root_opt {
                                                dialog = dialog.set_directory(&root);
                                            }
                                            if let Some(path) = dialog.save_file() {
                                                let _ =
                                                    proxy.send_event(AppEvent::SaveNewFile(path));
                                            }
                                        });
                                    }
                                    MenuItem::NewFolder => {
                                        let proxy = event_proxy.clone();
                                        let root_opt = sidebar.root_folder.clone();
                                        std::thread::spawn(move || {
                                            let mut dialog =
                                                rfd::FileDialog::new().set_title("New Folder");
                                            if let Some(root) = root_opt {
                                                dialog = dialog.set_directory(&root);
                                            }
                                            if let Some(path) = dialog.save_file() {
                                                let _ =
                                                    proxy.send_event(AppEvent::CreateFolder(path));
                                            }
                                        });
                                    }
                                    MenuItem::OpenFile => {
                                        let proxy = event_proxy.clone();
                                        std::thread::spawn(move || {
                                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                let _ = proxy.send_event(AppEvent::OpenFile(path));
                                            }
                                        });
                                    }
                                    MenuItem::OpenFolder => {
                                        let proxy = event_proxy.clone();
                                        std::thread::spawn(move || {
                                            if let Some(dir) = rfd::FileDialog::new().pick_folder()
                                            {
                                                let _ = proxy.send_event(AppEvent::OpenFolder(dir));
                                            }
                                        });
                                    }
                                    MenuItem::Save => {
                                        if let Some(tab) = tabs.active_tab_mut() {
                                            let _ = tab.buffer.save();
                                            if let Some(p) = &tab.buffer.file_path {
                                                if let Some(name) =
                                                    p.file_name().and_then(|n| n.to_str())
                                                {
                                                    tab.title = name.to_string();
                                                }
                                            }
                                            if sidebar.root_folder.is_some() {
                                                sidebar.refresh_folder();
                                            }
                                            update_window_title(
                                                &window,
                                                &tabs,
                                                &sidebar,
                                                &mut current_title,
                                            );
                                            window.request_redraw();
                                        }
                                    }
                                },
                                ActionEvent::OpenFile(path) => {
                                    tabs.open_file(path);
                                    update_window_title(
                                        &window,
                                        &tabs,
                                        &sidebar,
                                        &mut current_title,
                                    );
                                    window.request_redraw();
                                }
                                ActionEvent::SaveTab(idx) => {
                                    if let Some(tab) = tabs.tabs.get_mut(idx) {
                                        let _ = tab.buffer.save();
                                    }
                                    if sidebar.root_folder.is_some() {
                                        sidebar.refresh_folder();
                                    }
                                    tabs.close_tab(idx);
                                    update_window_title(
                                        &window,
                                        &tabs,
                                        &sidebar,
                                        &mut current_title,
                                    );
                                    window.request_redraw();
                                }
                                ActionEvent::Redraw => {
                                    update_window_title(
                                        &window,
                                        &tabs,
                                        &sidebar,
                                        &mut current_title,
                                    );
                                    window.request_redraw();
                                }
                                ActionEvent::None => {}
                            }

                            let current_layout = renderer.layout(total_lines, sidebar.width);
                            let desired_icon =
                                input.desired_cursor_icon(&current_layout, &tabs, &sidebar);
                            if active_cursor_icon != desired_icon {
                                active_cursor_icon = desired_icon;
                                window.set_cursor_icon(desired_icon);
                            }
                        }

                        WindowEvent::Focused(is_focused) => {
                            if !is_focused {
                                input.drag = input::DragState::None;
                            }
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            if input.handle_mouse_wheel(
                                delta,
                                &mut tabs,
                                &mut sidebar,
                                &layout,
                                cw,
                                lh,
                                renderer.height,
                            ) {
                                window.request_redraw();
                            }
                        }

                        WindowEvent::KeyboardInput { event, .. } => {
                            if input.handle_key(&event, &mut tabs, &layout, &mut clipboard) {
                                if sidebar.root_folder.is_some() {
                                    sidebar.refresh_folder();
                                }
                                update_window_title(&window, &tabs, &sidebar, &mut current_title);
                                window.request_redraw();
                            }
                        }

                        WindowEvent::CloseRequested => elwt.exit(),
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .expect("Error running event loop");
}
