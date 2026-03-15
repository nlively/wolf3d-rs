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

/// Sprite codes in plane 1 that indicate actor spawns.
/// These match GFXV_WL6.H values.
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

    pub const DOOR_H: u16 = 90;
    pub const DOOR_V: u16 = 91;
    pub const GOLD_DOOR_H: u16 = 92;
    pub const GOLD_DOOR_V: u16 = 93;
    pub const SILVER_DOOR_H: u16 = 94;
    pub const SILVER_DOOR_V: u16 = 95;
    pub const ELEVATOR_H: u16 = 100;
    pub const ELEVATOR_V: u16 = 101;
}

pub struct GameMap {
    pub level: Level,
}

impl GameMap {
    pub fn from_level(level: Level) -> Self {
        Self { level }
    }

    /// Parse plane 1 spawn codes and populate actors, doors, and initial player pos.
    pub fn spawn_things(
        &self,
        actors: &mut ActorList,
        doors: &mut DoorList,
    ) -> Player {
        use spawn_codes::*;
        let mut player = Player::new();

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

                    DOOR_H => doors.doors.push(Door::new(x, y, DoorKind::Normal, true)),
                    DOOR_V => doors.doors.push(Door::new(x, y, DoorKind::Normal, false)),
                    GOLD_DOOR_H => doors.doors.push(Door::new(x, y, DoorKind::Gold, true)),
                    GOLD_DOOR_V => doors.doors.push(Door::new(x, y, DoorKind::Gold, false)),
                    SILVER_DOOR_H => doors.doors.push(Door::new(x, y, DoorKind::Silver, true)),
                    SILVER_DOOR_V => doors.doors.push(Door::new(x, y, DoorKind::Silver, false)),
                    ELEVATOR_H => doors.doors.push(Door::new(x, y, DoorKind::Elevator, true)),
                    ELEVATOR_V => doors.doors.push(Door::new(x, y, DoorKind::Elevator, false)),

                    _ => {} // TODO: remaining static objects / enemy types
                }
            }
        }

        player
    }
}
