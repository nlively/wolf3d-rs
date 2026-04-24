/// In-game map — wraps the raw Level data with spawn logic.
///
/// Corresponds to SetupGameLevel() in WL_GAME.C and SpawnThings() in WL_ACT1.C.
use crate::assets::maps::Level;
use crate::game::{
    actor::{ActorKind, ActorList},
    door::{Door, DoorKind, DoorList},
    player::Player,
};
use crate::math::{tables::FINEANGLES, Fixed};

/// Sprite codes in plane 1 that indicate actor/player spawns.
mod spawn_codes {
    pub const PLAYER_N: u16 = 19;
    pub const PLAYER_E: u16 = 20;
    pub const PLAYER_S: u16 = 21;
    pub const PLAYER_W: u16 = 22;

    pub const GUARD_N: u16 = 108;
    pub const GUARD_E: u16 = 109;
    pub const GUARD_S: u16 = 110;
    pub const GUARD_W: u16 = 111;

    pub const DOG_N: u16 = 138;
    pub const DOG_E: u16 = 139;
    pub const DOG_S: u16 = 140;
    pub const DOG_W: u16 = 141;
}

pub struct GameMap {
    pub level: Level,
}

impl GameMap {
    pub fn from_level(level: Level) -> Self {
        Self { level }
    }

    /// Parse plane 1 spawn codes for actors/player, and door_spawns for doors.
    pub fn spawn_things(
        &self,
        actors: &mut ActorList,
        doors: &mut DoorList,
    ) -> Player {
        use spawn_codes::*;
        let mut player = Player::new();

        // Actors and player from plane 1.
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let code = self.level.sprite_at(x, y);
                if code == 0 {
                    continue;
                }
                let fx = Fixed::from_int(x as i32) + Fixed::from_f32(0.5);
                let fy = Fixed::from_int(y as i32) + Fixed::from_f32(0.5);

                match code {
                    PLAYER_N => player = Player::spawn(x, y, 0),
                    PLAYER_E => player = Player::spawn(x, y, FINEANGLES / 4),
                    PLAYER_S => player = Player::spawn(x, y, FINEANGLES / 2),
                    PLAYER_W => player = Player::spawn(x, y, FINEANGLES * 3 / 4),

                    GUARD_N | GUARD_E | GUARD_S | GUARD_W => {
                        actors.spawn(ActorKind::Guard, fx, fy);
                    }
                    DOG_N | DOG_E | DOG_S | DOG_W => {
                        actors.spawn(ActorKind::Dog, fx, fy);
                    }

                    _ => {} // TODO: remaining static objects / enemy types
                }
            }
        }

        // Doors from plane 0 (pre-parsed into door_spawns at load time).
        for ds in &self.level.door_spawns {
            let kind = match ds.lock {
                1 => DoorKind::Gold,
                2 => DoorKind::Silver,
                5 => DoorKind::Elevator,
                _ => DoorKind::Normal,
            };
            doors.doors.push(Door::new(ds.tile_x, ds.tile_y, kind, ds.is_vertical));
        }

        player
    }
}
