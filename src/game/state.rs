/// Top-level game state — corresponds to WL_PLAY.C / WL_GAME.C.
///
/// Drives the main game loop: ticking all actors, updating doors,
/// checking win/lose conditions, and orchestrating drawing.
use std::path::Path;

use crate::assets::graphics::GraphicsCache;
use crate::engine::renderer::{Renderer, View};
use crate::game::{actor::ActorList, door::DoorList, map::GameMap, player::Player};
use crate::input::handler::InputHandler;

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
    pub graphics: Option<GraphicsCache>,
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
            graphics: None,
            episode: 0,
            map_num: 0,
            tick: 0,
        }
    }

    /// Load all asset data from `base` (e.g. `assets/`).
    /// Call once at startup before the first frame.
    pub fn load_assets(&mut self, base: &Path) {
        match GraphicsCache::load(base) {
            Ok(gc) => self.graphics = Some(gc),
            Err(e) => log::error!("failed to load graphics: {e}"),
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

    fn draw_game(&mut self, fb: &mut [u8], width: usize, _height: usize) {
        let view = View {
            x: self.player.x,
            y: self.player.y,
            angle: self.player.angle,
        };

        let textures: &[Vec<u8>] = self
            .graphics
            .as_ref()
            .map(|g| g.wall_textures.as_slice())
            .unwrap_or(&[]);

        // Build a flat door-positions slice (indexed by door number).
        let door_pos: Vec<u8> = self.doors.doors.iter().map(|d| d.position).collect();

        if let Some(map) = &self.map {
            self.renderer.draw_frame(fb, width, &view, &map.level, textures, &door_pos);
        } else {
            fb.fill(0);
        }

        // TODO: draw sprites, HUD
    }

    fn draw_menu(&self, fb: &mut [u8], _width: usize, _height: usize) {
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
