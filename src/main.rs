mod app;
mod config;
mod editor;
mod input;
mod session;
mod sidebar;
mod syntax;
mod terminal;
mod ui;

use app::App;
use input::AppEvent;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let event_proxy = event_loop.create_proxy();
    let mut app = App::new(event_proxy);
    app.apply_loaded_session();
    event_loop
        .run_app(&mut app)
        .expect("Error running event loop");
}
