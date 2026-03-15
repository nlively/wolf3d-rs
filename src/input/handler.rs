/// Input handler — corresponds to ID_IN.C.
///
/// Translates raw winit key events into abstract game actions.
/// Key bindings are intentional defaults matching the original game;
/// they will be made configurable in a later milestone.
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Default)]
pub struct InputHandler {
    pub forward: bool,
    pub back: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub strafe_left: bool,
    pub strafe_right: bool,
    pub fire: bool,
    pub open: bool,      // "use" key — opens doors
    pub run: bool,
    pub start: bool,     // Enter / confirm
    pub escape: bool,
    pub quit: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let PhysicalKey::Code(code) = event.physical_key else { return };

        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => self.forward = pressed,
            KeyCode::ArrowDown | KeyCode::KeyS => self.back = pressed,
            KeyCode::ArrowLeft => self.turn_left = pressed,
            KeyCode::ArrowRight => self.turn_right = pressed,
            KeyCode::KeyA => self.strafe_left = pressed,
            KeyCode::KeyD => self.strafe_right = pressed,
            KeyCode::ControlLeft | KeyCode::Space => self.fire = pressed,
            KeyCode::KeyE | KeyCode::Enter => {
                if code == KeyCode::Enter && pressed {
                    self.start = true;
                }
                if code == KeyCode::KeyE {
                    self.open = pressed;
                }
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.run = pressed,
            KeyCode::Escape => {
                self.escape = pressed;
                if pressed {
                    self.quit = true;
                }
            }
            _ => {}
        }
    }

    /// Clear single-frame events (e.g. "start pressed this frame").
    pub fn clear_events(&mut self) {
        self.start = false;
    }

    pub fn move_forward(&self) -> bool {
        self.forward
    }

    pub fn move_back(&self) -> bool {
        self.back
    }

    pub fn turn_left(&self) -> bool {
        self.turn_left
    }

    pub fn turn_right(&self) -> bool {
        self.turn_right
    }

    pub fn strafe_left(&self) -> bool {
        self.strafe_left
    }

    pub fn strafe_right(&self) -> bool {
        self.strafe_right
    }

    pub fn fire(&self) -> bool {
        self.fire
    }

    pub fn open_door(&self) -> bool {
        self.open
    }

    pub fn start_pressed(&self) -> bool {
        self.start
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }
}
