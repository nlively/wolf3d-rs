/// Player — corresponds to WL_AGENT.C.
///
/// Handles movement, collision, weapon logic, and pickup detection.
use crate::assets::maps::MAP_SIZE;
use crate::game::map::GameMap;
use crate::input::handler::InputHandler;
use crate::math::{tables::FINEANGLES, Fixed};

pub const MOVE_SPEED: i32 = 6;   // tiles per second (approx)
pub const TURN_SPEED: usize = 40; // fine-angles per frame

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Knife = 0,
    Pistol = 1,
    MachineGun = 2,
    ChainGun = 3,
}

pub struct Player {
    /// Position in tile-space (fixed-point, 1 unit = 1 tile).
    pub x: Fixed,
    pub y: Fixed,
    /// View angle in fine-angle units (0..FINEANGLES).
    pub angle: usize,
    pub health: i32,
    pub ammo: i32,
    pub score: u32,
    pub lives: i32,
    pub weapon: Weapon,
    pub face_dir: usize, // for status bar face sprite
}

impl Player {
    pub fn new() -> Self {
        Self {
            x: Fixed::from_int(1),
            y: Fixed::from_int(1),
            angle: 0,
            health: 100,
            ammo: 8,
            score: 0,
            lives: 3,
            weapon: Weapon::Pistol,
            face_dir: 0,
        }
    }

    /// Spawn the player at the given tile with facing direction.
    pub fn spawn(x: usize, y: usize, angle: usize) -> Self {
        Self {
            x: Fixed::from_int(x as i32) + Fixed::from_f32(0.5),
            y: Fixed::from_int(y as i32) + Fixed::from_f32(0.5),
            angle,
            ..Self::new()
        }
    }

    pub fn update(&mut self, input: &InputHandler, map: &GameMap) {
        // Turn
        if input.turn_left() {
            self.angle = (self.angle + FINEANGLES - TURN_SPEED) % FINEANGLES;
        }
        if input.turn_right() {
            self.angle = (self.angle + TURN_SPEED) % FINEANGLES;
        }

        // Move forward/back
        let cos = Fixed::from_f32((self.angle as f32 * std::f32::consts::TAU / FINEANGLES as f32).cos());
        let sin = Fixed::from_f32((self.angle as f32 * std::f32::consts::TAU / FINEANGLES as f32).sin());
        let speed = Fixed::from_f32(0.1);

        if input.move_forward() {
            let new_x = self.x + cos * speed;
            let new_y = self.y + sin * speed;
            self.try_move(new_x, new_y, map);
        }
        if input.move_back() {
            let new_x = self.x - cos * speed;
            let new_y = self.y - sin * speed;
            self.try_move(new_x, new_y, map);
        }
        if input.strafe_left() {
            let new_x = self.x + sin * speed;
            let new_y = self.y - cos * speed;
            self.try_move(new_x, new_y, map);
        }
        if input.strafe_right() {
            let new_x = self.x - sin * speed;
            let new_y = self.y + cos * speed;
            self.try_move(new_x, new_y, map);
        }
    }

    /// Attempt to move to (new_x, new_y), with simple axis-separated collision.
    fn try_move(&mut self, new_x: Fixed, new_y: Fixed, map: &GameMap) {
        let tx = new_x.to_int() as usize;
        let ty = new_y.to_int() as usize;
        if tx < MAP_SIZE && ty < MAP_SIZE && !map.level.is_solid_wall(tx, ty) {
            self.x = new_x;
            self.y = new_y;
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
