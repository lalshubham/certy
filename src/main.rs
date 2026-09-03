mod buffer;
mod config;
mod font;
mod history;
mod input;
mod layout;
mod renderer;

use buffer::EditorBuffer;
use input::InputHandler;
use renderer::Renderer;
use std::sync::Arc;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorIcon, WindowBuilder},
};

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Certy")
            .with_inner_size(LogicalSize::new(1024.0, 768.0))
            .build(&event_loop)
            .expect("Failed to create window"),
    );

    let mut renderer = Renderer::new(window.clone());
    let mut buffer = EditorBuffer::new();
    let mut input = InputHandler::default();
    let mut clipboard = arboard::Clipboard::new().ok();
    let mut active_cursor_icon = CursorIcon::Default;

    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            if let Event::WindowEvent { event, window_id } = event {
                if window_id != window.id() {
                    return;
                }

                let total_lines = buffer.text().len_lines();
                let layout = renderer.layout(total_lines);
                let cw = renderer.font_manager.char_width;
                let lh = renderer.font_manager.line_height;

                match event {
                    WindowEvent::RedrawRequested => renderer.render(&buffer),
                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width, size.height);
                        let l = renderer.layout(buffer.text().len_lines());
                        buffer.fit_view(l.visible_lines, l.visible_cols);
                        window.request_redraw();
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        input.modifiers = modifiers.state();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let desired_icon = input.desired_cursor_icon(&layout);
                        if active_cursor_icon != desired_icon {
                            active_cursor_icon = desired_icon;
                            window.set_cursor_icon(desired_icon);
                        }

                        if input.handle_cursor_move(
                            position.x,
                            position.y,
                            &mut buffer,
                            &layout,
                            cw,
                            lh,
                        ) {
                            window.request_redraw();
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if input.handle_mouse_click(
                            state,
                            button,
                            &mut buffer,
                            &layout,
                            cw,
                            lh,
                            renderer.width,
                            renderer.height,
                        ) {
                            window.request_redraw();
                        }
                    }
                    WindowEvent::Focused(is_focused) => {
                        if !is_focused {
                            input.drag = input::DragState::None;
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        if input.handle_mouse_wheel(delta, &mut buffer, &layout, cw, lh) {
                            window.request_redraw();
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if input.handle_key(&event, &mut buffer, &layout, &mut clipboard) {
                            window.request_redraw();
                        }
                    }
                    WindowEvent::CloseRequested => elwt.exit(),
                    _ => {}
                }
            }
        })
        .expect("Error running event loop");
}
