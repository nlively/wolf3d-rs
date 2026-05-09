use std::sync::Arc;

use anyhow::Result;
use log::info;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use wolf3d_rs::game::state::GameState;
use wolf3d_rs::input::handler::InputHandler;

const SCREEN_WIDTH: u32 = 320;
const SCREEN_HEIGHT: u32 = 200;
const SCALE: u32 = 3;

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    game: GameState,
    input: InputHandler,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            game: GameState::new(),
            input: InputHandler::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let size = LogicalSize::new(SCREEN_WIDTH * SCALE, SCREEN_HEIGHT * SCALE);
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Wolfenstein 3D")
                        .with_inner_size(size)
                        .with_min_inner_size(LogicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT)),
                )
                .expect("failed to create window"),
        );

        let inner_size = window.inner_size();
        // Arc<Window> is 'static, so SurfaceTexture<'static, Arc<Window>> compiles.
        let surface_texture =
            SurfaceTexture::new(inner_size.width, inner_size.height, Arc::clone(&window));
        let pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)
            .expect("failed to create pixel buffer");

        self.window = Some(window);
        self.pixels = Some(pixels);

        info!("Window created: {}x{} ({}x scale)", SCREEN_WIDTH, SCREEN_HEIGHT, SCALE);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.handle_key(&event);
                if self.input.quit_requested() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    pixels.resize_surface(size.width, size.height).ok();
                }
            }
            WindowEvent::RedrawRequested => {
                // Update game state
                self.game.tick(&self.input);
                self.input.clear_events();

                // Draw frame
                if let Some(pixels) = self.pixels.as_mut() {
                    let frame = pixels.frame_mut();
                    self.game.draw(frame, SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);

                    if pixels.render().is_err() {
                        event_loop.exit();
                    }
                }

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    info!("wolf3d-rs starting");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();

    let path = std::path::Path::new("assets");
    println!("loading assets");
    app.game.load_assets(path);

    event_loop.run_app(&mut app)?;

    Ok(())
}
