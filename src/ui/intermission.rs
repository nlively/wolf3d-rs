/// Intermission / level complete screen — corresponds to WL_INTER.C.
///
/// Shows time, kill ratio, secret ratio, and treasure ratio for the level.

pub struct LevelStats {
    pub time_secs: u32,
    pub kills: u32,
    pub total_kills: u32,
    pub secrets: u32,
    pub total_secrets: u32,
    pub treasure: u32,
    pub total_treasure: u32,
    pub par_time_secs: u32,
}

pub struct Intermission {
    pub stats: LevelStats,
    /// Ticks since the screen started showing (drives the counting animation).
    pub ticks: u32,
}

impl Intermission {
    pub fn new(stats: LevelStats) -> Self {
        Self { stats, ticks: 0 }
    }

    pub fn tick(&mut self) {
        self.ticks += 1;
    }

    pub fn draw(&self, fb: &mut [u8], width: usize, height: usize) {
        // Clear to dark background
        for chunk in fb.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[0x00, 0x00, 0x00, 0xFF]);
        }
        // TODO: blit level stats using game font
    }
}
