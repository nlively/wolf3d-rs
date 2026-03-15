/// Actor system — corresponds to WL_ACT1.C (statics/doors) and WL_ACT2.C (enemies).
///
/// Every interactive object in the world is an `Actor`.  The original used a
/// linked list of `objtype` structs; we use a Vec with an optional free-list.
use crate::math::Fixed;

/// All actor (enemy / object) kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    // Enemies
    Guard,
    Officer,
    SS,
    Dog,
    Mutant,
    // Bosses
    Hans,
    Schabbs,
    Gretel,
    KettleBody,
    Hitler,
    Angel,
    // Static objects (items, decorations)
    GoldKey,
    SilverKey,
    FoodItem,
    MedKit,
    Ammo,
    MachineGun,
    ChainGun,
    Cross,
    Chalice,
    Bible,
    Crown,
    // Decorations
    Barrel,
    TableChairs,
    CeilingLight,
    // … add more as needed
}

/// An actor's high-level state in the AI state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorMode {
    Stand,
    Path,
    Chase,
    Shoot,
    Pain,
    Die,
    Dead,
    Static, // non-AI objects (items, decorations)
}

/// Direction an actor is facing (8-way).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    None,
}

pub struct Actor {
    pub kind: ActorKind,
    pub mode: ActorMode,
    pub x: Fixed,
    pub y: Fixed,
    pub dir: Dir,
    pub angle: i32,
    pub health: i32,
    pub speed: Fixed,
    /// Index into the sprite table.
    pub sprite: usize,
    /// Tics until the current animation frame advances.
    pub tic_count: i32,
    /// Distance remaining to move this step (used for path actors).
    pub dist: Fixed,
    /// Has this actor spotted the player?
    pub flags: u32,
}

pub const FLAG_FIRSTATTACK: u32 = 0x01;
pub const FLAG_AMBUSH: u32 = 0x02;

impl Actor {
    pub fn new(kind: ActorKind, x: Fixed, y: Fixed) -> Self {
        Self {
            kind,
            mode: ActorMode::Stand,
            x,
            y,
            dir: Dir::None,
            angle: 0,
            health: 25,
            speed: Fixed::from_f32(0.05),
            sprite: 0,
            tic_count: 0,
            dist: Fixed::ZERO,
            flags: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        !matches!(self.mode, ActorMode::Dead)
    }
}

pub struct ActorList {
    pub actors: Vec<Actor>,
}

impl ActorList {
    pub fn new() -> Self {
        Self { actors: Vec::new() }
    }

    pub fn spawn(&mut self, kind: ActorKind, x: Fixed, y: Fixed) -> usize {
        let id = self.actors.len();
        self.actors.push(Actor::new(kind, x, y));
        id
    }

    /// Tick all actors — delegates to the AI module.
    pub fn update_all(&mut self, player: &crate::game::player::Player) {
        use crate::game::ai;
        for actor in &mut self.actors {
            if actor.is_alive() {
                ai::think(actor, player);
            }
        }
    }
}

impl Default for ActorList {
    fn default() -> Self {
        Self::new()
    }
}
