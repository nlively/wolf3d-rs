/// Top-level game state — corresponds to WL_PLAY.C / WL_GAME.C.
///
/// Drives the main game loop: ticking all actors, updating doors,
/// checking win/lose conditions, and orchestrating drawing.
use crate::engine::{
    renderer::{Renderer, View},
    scaler,
};
use crate::game::{actor::ActorList, door::DoorList, map::GameMap, player::Player};
use crate::input::handler::InputHandler;
use crate::math::Fixed;

pub enum Screen {
    /// Main game view.
    Playing,
    /// Title / main menu.
    Menu,
    /// Between-level stats.
    Intermission,
    /// Game over.
    GameOver,
}

pub struct GameState {
    pub screen: Screen,
    pub player: Player,
    pub map: Option<GameMap>,
    pub actors: ActorList,
    pub doors: DoorList,
    pub renderer: Renderer,
    /// Current episode (0-based) and map number (0-based within episode).
    pub episode: usize,
    pub map_num: usize,
    pub tick: u64,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            player: Player::new(),
            map: None,
            actors: ActorList::new(),
            doors: DoorList::new(),
            renderer: Renderer::new(),
            episode: 0,
            map_num: 0,
            tick: 0,
        }
    }

    /// Called once per frame — update all game objects.
    pub fn tick(&mut self, input: &InputHandler) {
        self.tick += 1;

        match self.screen {
            Screen::Playing => {
                if let Some(map) = &self.map {
                    self.player.update(input, map);
                }
                self.doors.update();
                self.actors.update_all(&self.player);
            }
            Screen::Menu => {
                // TODO: menu logic
                // For now, pressing Enter starts the game.
                if input.start_pressed() {
                    self.screen = Screen::Playing;
                }
            }
            Screen::Intermission | Screen::GameOver => {}
        }
    }

    /// Draw the current screen into the RGBA8888 framebuffer.
    pub fn draw(&mut self, fb: &mut [u8], width: usize, height: usize) {
        match self.screen {
            Screen::Playing => self.draw_game(fb, width, height),
            Screen::Menu => self.draw_menu(fb, width, height),
            Screen::Intermission => {}
            Screen::GameOver => {}
        }
    }

    fn draw_game(&mut self, fb: &mut [u8], width: usize, height: usize) {
        let view = View {
            x: self.player.x,
            y: self.player.y,
            angle: self.player.angle,
        };

        if let Some(map) = &self.map {
            self.renderer.draw_frame(fb, width, &view, &map.level, &[]);
        } else {
            // No map loaded — clear to black
            fb.fill(0);
        }

        // TODO: draw sprites, HUD
    }

    fn draw_menu(&self, fb: &mut [u8], width: usize, _height: usize) {
        // Placeholder: grey screen with a note.
        for chunk in fb.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[0x1C, 0x1C, 0x1C, 0xFF]);
        }
        // TODO: blit actual menu graphics
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
