/// Pre-calculated trigonometry tables, matching the originals in WL_DRAW.C.
///
/// The original used fixed-point tables with FINEANGLES=3600 subdivisions.
/// We generate these at startup rather than embedding them.
use crate::math::Fixed;

pub const FINEANGLES: usize = 3600;
pub const ANG90: usize = FINEANGLES / 4;
pub const ANG180: usize = FINEANGLES / 2;
pub const ANG270: usize = FINEANGLES * 3 / 4;
pub const ANG360: usize = FINEANGLES;

pub const TILEGLOBAL: i32 = 1 << 16; // Fixed::ONE in tile units
pub const TILESHIFT: i32 = 16;

/// All trig tables indexed by fine angle (0..FINEANGLES).
pub struct TrigTables {
    pub sin: Vec<Fixed>,
    pub cos: Vec<Fixed>,
    pub tan: Vec<Fixed>,
    /// finetangent used for wall-fish calculations in the raycaster
    pub finetangent: Vec<Fixed>,
}

impl TrigTables {
    pub fn build() -> Self {
        let mut sin = Vec::with_capacity(FINEANGLES);
        let mut cos = Vec::with_capacity(FINEANGLES);
        let mut tan = Vec::with_capacity(FINEANGLES);
        let mut finetangent = Vec::with_capacity(FINEANGLES);

        for i in 0..FINEANGLES {
            let radians = (i as f64) * std::f64::consts::TAU / (FINEANGLES as f64);
            sin.push(Fixed::from_f32(radians.sin() as f32));
            cos.push(Fixed::from_f32(radians.cos() as f32));

            let t = radians.tan();
            // Clamp to avoid infinity at 90/270 degrees
            let t_clamped = t.clamp(-32767.0, 32767.0);
            tan.push(Fixed::from_f32(t_clamped as f32));
            finetangent.push(Fixed::from_f32(t_clamped as f32));
        }

        TrigTables { sin, cos, tan, finetangent }
    }

    pub fn sin(&self, angle: usize) -> Fixed {
        self.sin[angle % FINEANGLES]
    }

    pub fn cos(&self, angle: usize) -> Fixed {
        self.cos[angle % FINEANGLES]
    }

    pub fn tan(&self, angle: usize) -> Fixed {
        self.tan[angle % FINEANGLES]
    }
}
