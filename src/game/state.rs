/// Top-level game state — corresponds to WL_PLAY.C / WL_GAME.C.
///
/// Drives the main game loop: ticking all actors, updating doors,
/// checking win/lose conditions, and orchestrating drawing.
use std::path::Path;

use crate::assets::graphics::GraphicsCache;
use crate::engine::renderer::{Renderer, View};
use crate::game::{actor::ActorList, door::DoorList, map::GameMap, player::Player};
use crate::input::handler::InputHandler;

#[derive(Debug)]
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

pub enum MenuOption {
    NewGame,
    Sound,
    Control,
    LoadGame,
    SaveGame,
    ChangeView,
    #[cfg(all(not(goodtimes), not(spear)))]
    ReadThis,
    ViewScores,
    BackToDemo,
    Quit,
}

const MENU_X: i16 = 76;
const MENU_Y: i16 = 55;
const MENU_W: i16 = 178;
#[cfg(not(spear))]
const MENU_H: i16 = 13*10+6;
#[cfg(spear)]
const MENU_H: i16 = 13*9+6;

const SM_X: i16 = 48;
const SM_W: i16 = 250;

const SM_Y1: u16 = 20;
const SM_H1: u16 = 4*13-7;
const SM_Y2: u16 = SM_Y1+5*13;
const SM_H2: u16 = 4*13-7;
const SM_Y3: u16 = SM_Y2+5*13;
const SM_H3: u16 = 3*13-7;

const CTL_X: u16 = 24;
const CTL_Y: u16 = 70;
const CTL_W: u16 = 284;
const CTL_H: u16 = 13*7-7;

const LSM_X: u16 = 85;
const LSM_Y: u16 = 55;
const LSM_W: u16 = 175;
const LSM_H: u16 = 10*13+10;

const NM_X: u16 = 50;
const NM_Y: u16 = 100;
const NM_W: u16 = 225;
const NM_H: u16 = 13*4+15;

const NE_X: u16 = 10;
const NE_Y: u16 = 23;
const NE_W: u16 = 320-NE_X*2;
const NE_H: u16 = 200-NE_Y*2;

const CST_X: u16 = 20;
const CST_Y: u16 = 48;
const CST_START: u16 = 60;
const CST_SPC: u16 = 60;

#[cfg(any(spear, goodtimes))]
const STARTITEM:MenuOption = MenuOption::NewGame;

#[cfg(all(not(spear), not(goodtimes)))]
const STARTITEM:MenuOption = MenuOption::ReadThis;

pub struct MenuItemInfo {
    x: i16,
    y: i16,
    amount: u16,
    current_position: MenuOption,
    indent: u16,
}

impl MenuItemInfo {
    fn new(x: i16, y: i16, amount: u16, current_position: MenuOption, indent: u16) -> Self {
        Self {
            x, 
            y,
            amount,
            current_position,
            indent,
        }
    }
}

pub struct MenuItem {
    active: bool,
    title: String,
    handler: fn(i32),
}

impl MenuItem {
    fn new(active: bool, title: String, handler: fn(i32)) -> Self {
        Self {
            active,
            title,
            handler,
        }
    }
}

pub struct Menu {
    items: Vec<MenuItem>,
    item_info: Vec<MenuItemInfo>,
}

impl Menu {
    fn new() -> Self {
        let items = vec![
            MenuItem::new(true, String::from("New Game"), Self::new_game),
        ];

        let item_info = vec![
            MenuItemInfo::new(MENU_X, MENU_Y, 10, STARTITEM, 24),
        ];

        Self {
            items,
            item_info,
        }
    }

    fn new_game(i: i32) {

    }
}

fn handle_menu(item_i: &MenuItemInfo, items: [MenuItem], handler: fn(i16)) -> u16 {
    let key: u8;
    static mut redrawitem: i16 = 1;
    static mut lastitem: i16 = -1;
    let i: i16;
    let x: i16;
    let basey: i16;
    let exit: i16;
    let which: MenuOption;
    let shape: i16;
    let timer: i16;
    // let ci: ControlInfo;

    which = item_i.current_position;
    x = item_i.x & -8;
    basey = item_i.y - 2;
    y = basey + which*13;
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
    menu: Menu,
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
            menu: Menu::new(),
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
