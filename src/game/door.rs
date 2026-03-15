/// Door system — corresponds to the door logic in WL_ACT1.C.
///
/// Wolf3D supports up to 64 doors per level.  Each door is either:
///   - a normal hinged door (slides open sideways)
///   - a locked door (requires gold or silver key)
///   - an elevator tile (level exit)
///
/// Doors are drawn by the raycaster as a half-tile-offset wall segment.
pub const MAX_DOORS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorKind {
    Normal,
    Gold,   // requires gold key
    Silver, // requires silver key
    Elevator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorState {
    Closed,
    Opening,
    Open,
    Closing,
}

pub struct Door {
    pub tile_x: usize,
    pub tile_y: usize,
    pub kind: DoorKind,
    pub state: DoorState,
    /// 0 = fully closed, 63 = fully open.
    pub position: u8,
    /// Ticks until next state change.
    pub tic_count: i32,
    /// True if a horizontal door (E-W), false if vertical (N-S).
    pub horizontal: bool,
}

const DOOR_OPEN_TICKS: i32 = 300; // ticks before auto-close

impl Door {
    pub fn new(tile_x: usize, tile_y: usize, kind: DoorKind, horizontal: bool) -> Self {
        Self {
            tile_x,
            tile_y,
            kind,
            state: DoorState::Closed,
            position: 0,
            tic_count: 0,
            horizontal,
        }
    }

    pub fn open(&mut self) {
        if self.state == DoorState::Closed {
            self.state = DoorState::Opening;
        }
    }

    pub fn update(&mut self) {
        match self.state {
            DoorState::Opening => {
                if self.position < 63 {
                    self.position += 4;
                } else {
                    self.position = 63;
                    self.state = DoorState::Open;
                    self.tic_count = DOOR_OPEN_TICKS;
                }
            }
            DoorState::Open => {
                self.tic_count -= 1;
                if self.tic_count <= 0 {
                    self.state = DoorState::Closing;
                }
            }
            DoorState::Closing => {
                if self.position > 0 {
                    self.position -= 4;
                } else {
                    self.position = 0;
                    self.state = DoorState::Closed;
                }
            }
            DoorState::Closed => {}
        }
    }

    /// How far open the door is as a fixed-point fraction (0.0 = closed, 1.0 = open).
    pub fn openness(&self) -> f32 {
        self.position as f32 / 63.0
    }
}

pub struct DoorList {
    pub doors: Vec<Door>,
}

impl DoorList {
    pub fn new() -> Self {
        Self { doors: Vec::new() }
    }

    pub fn update(&mut self) {
        for door in &mut self.doors {
            door.update();
        }
    }

    pub fn at(&self, tile_x: usize, tile_y: usize) -> Option<&Door> {
        self.doors.iter().find(|d| d.tile_x == tile_x && d.tile_y == tile_y)
    }

    pub fn at_mut(&mut self, tile_x: usize, tile_y: usize) -> Option<&mut Door> {
        self.doors.iter_mut().find(|d| d.tile_x == tile_x && d.tile_y == tile_y)
    }
}

impl Default for DoorList {
    fn default() -> Self {
        Self::new()
    }
}
