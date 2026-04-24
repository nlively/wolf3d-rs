/// Raycasting renderer — direct port of WL_DRAW.C.
///
/// # Architecture
///
/// Wolf3D uses a classic DDA-based raycaster:
///   1. For each screen column, cast a ray from the player position in the
///      direction determined by the player angle + column offset.
///   2. Step along the ray in the tile grid until hitting a wall (DDA).
///   3. Compute the projected wall height from the perpendicular distance.
///   4. Draw the wall column, floor, and ceiling.
///   5. After all walls are drawn, sort and draw sprites.
///
/// All distances use 16.16 fixed-point arithmetic.
use crate::assets::maps::Level;
use crate::math::{
    tables::{TrigTables, FINEANGLES},
    Fixed,
};

pub const VIEW_WIDTH: usize = 256;
pub const VIEW_HEIGHT: usize = 152;
pub const HALF_HEIGHT: usize = VIEW_HEIGHT / 2;

/// VSWAP wall texture index for the standard door face (PMSpriteStart - 8 = 106 - 8).
const DOOR_TEXTURE: u16 = 98;

/// Player view parameters passed to each frame.
pub struct View {
    pub x: Fixed,
    pub y: Fixed,
    /// Player angle in fine-angle units (0..FINEANGLES).
    pub angle: usize,
}

/// One entry per screen column produced by the raycaster.
#[derive(Default, Debug)]
struct ColumnHit {
    /// Perpendicular (fisheye-corrected) distance to the wall.
    dist: Fixed,
    texture: u16,
    /// Horizontal texture coordinate (0..63).
    tex_x: u8,
    /// True if the hit was on an E/W wall face; false for N/S.
    ew_face: bool,
}

pub struct Renderer {
    trig: TrigTables,
    columns: Vec<ColumnHit>,
    /// Depth buffer (perpendicular distances) used for sprite clipping.
    pub depth_buf: Vec<Fixed>,
}

impl Renderer {
    pub fn new() -> Self {
        let trig = TrigTables::build();
        let columns = (0..VIEW_WIDTH).map(|_| ColumnHit::default()).collect();
        let depth_buf = vec![Fixed::ZERO; VIEW_WIDTH];
        Self { trig, columns, depth_buf }
    }

    /// Draw a full frame into `fb`.
    ///
    /// `fb` is a flat RGBA8888 buffer of `stride * height` pixels.
    /// `textures`: 64×64 RGB (3 bytes/pixel) slabs indexed by wall tile number.
    /// `door_positions`: openness of each door by door index (0=closed, 63=open).
    pub fn draw_frame(
        &mut self,
        fb: &mut [u8],
        stride: usize,
        view: &View,
        level: &Level,
        textures: &[Vec<u8>],
        door_positions: &[u8],
    ) {
        self.draw_ceiling_floor(fb, stride);
        self.cast_walls(view, level, door_positions);
        self.draw_walls(fb, stride, textures);
    }

    fn draw_ceiling_floor(&self, fb: &mut [u8], stride: usize) {
        for y in 0..VIEW_HEIGHT {
            let color = if y < HALF_HEIGHT {
                [0x39, 0x39, 0x39, 0xFF]
            } else {
                [0x70, 0x70, 0x70, 0xFF]
            };
            for x in 0..VIEW_WIDTH {
                let offset = (y * stride + x) * 4;
                if offset + 3 < fb.len() {
                    fb[offset..offset + 4].copy_from_slice(&color);
                }
            }
        }
    }

    /// DDA raycaster.
    ///
    /// raw_dist (side_dist accumulator) is the Euclidean ray length.
    /// Perpendicular depth = raw_dist * cos(ray_angle - player_angle),
    /// which is what we store in ColumnHit::dist to avoid fisheye distortion.
    fn cast_walls(&mut self, view: &View, level: &Level, door_positions: &[u8]) {
        let fov_half = FINEANGLES / 12; // 60° FOV → ±30°
        let start_angle = (view.angle + FINEANGLES - fov_half) % FINEANGLES;

        for col in 0..VIEW_WIDTH {
            let ray_angle =
                (start_angle + col * (FINEANGLES / 6) / VIEW_WIDTH) % FINEANGLES;

            // Angle between this ray and forward — used for fisheye correction.
            let angle_diff = (ray_angle + FINEANGLES - view.angle) % FINEANGLES;

            let cos = self.trig.cos(ray_angle);
            let sin = self.trig.sin(ray_angle);

            let mut map_x = view.x.to_int();
            let mut map_y = view.y.to_int();

            let delta_dist_x = if cos == Fixed::ZERO {
                Fixed::from_int(64)
            } else {
                Fixed::abs(Fixed::from_int(1) / cos)
            };
            let delta_dist_y = if sin == Fixed::ZERO {
                Fixed::from_int(64)
            } else {
                Fixed::abs(Fixed::from_int(1) / sin)
            };

            let step_x: i32 = if cos < Fixed::ZERO { -1 } else { 1 };
            let step_y: i32 = if sin < Fixed::ZERO { -1 } else { 1 };

            let x_frac = Fixed(view.x.frac());
            let y_frac = Fixed(view.y.frac());
            let mut side_dist_x = if step_x < 0 {
                x_frac * delta_dist_x
            } else {
                (Fixed::ONE - x_frac) * delta_dist_x
            };
            let mut side_dist_y = if step_y < 0 {
                y_frac * delta_dist_y
            } else {
                (Fixed::ONE - y_frac) * delta_dist_y
            };

            const MAX_ITER: u8 = 100;
            let mut i: u8 = 0;

            loop {
                let ew_face: bool;
                let raw_dist: Fixed;
                if side_dist_x < side_dist_y {
                    ew_face = true;
                    raw_dist = side_dist_x;
                    map_x += step_x;
                    side_dist_x = side_dist_x + delta_dist_x;
                } else {
                    ew_face = false;
                    raw_dist = side_dist_y;
                    map_y += step_y;
                    side_dist_y = side_dist_y + delta_dist_y;
                }

                if map_x < 0 || map_y < 0 {
                    break;
                }

                let wall = level.wall_at(map_x as usize, map_y as usize);

                if wall & 0x80 != 0 {
                    // Door tile. Face sits at the tile centre (half a delta further).
                    let door_idx = (wall & 0x7F) as usize;
                    let openness = door_positions.get(door_idx).copied().unwrap_or(0);
                    let open_frac = Fixed(openness as i32 * 1024); // 0..Fixed::ONE

                    if ew_face {
                        let mid_dist = raw_dist + delta_dist_x / Fixed::from_int(2);
                        let y_at_door = view.y + mid_dist * sin;
                        let yf = Fixed(y_at_door.frac());
                        if yf.0 >= open_frac.0 {
                            let perp = mid_dist * self.trig.cos(angle_diff);
                            let tex_x = ((yf.0 - open_frac.0) >> 10).clamp(0, 63) as u8;
                            self.columns[col] = ColumnHit { dist: perp, texture: DOOR_TEXTURE, tex_x, ew_face };
                            self.depth_buf[col] = perp;
                            break;
                        }
                        // open enough — ray passes through, continue march
                    } else {
                        let mid_dist = raw_dist + delta_dist_y / Fixed::from_int(2);
                        let x_at_door = view.x + mid_dist * cos;
                        let xf = Fixed(x_at_door.frac());
                        if xf.0 >= open_frac.0 {
                            let perp = mid_dist * self.trig.cos(angle_diff);
                            let tex_x = ((xf.0 - open_frac.0) >> 10).clamp(0, 63) as u8;
                            self.columns[col] = ColumnHit { dist: perp, texture: DOOR_TEXTURE, tex_x, ew_face };
                            self.depth_buf[col] = perp;
                            break;
                        }
                    }
                } else if wall > 0 {
                    // Solid wall. Strip 0x40 jamb flag to get the texture index.
                    let perp = raw_dist * self.trig.cos(angle_diff);
                    let wall_texture = wall & !0x40u16;
                    let wall_hit = if ew_face {
                        view.y + raw_dist * sin
                    } else {
                        view.x + raw_dist * cos
                    };
                    self.columns[col] = ColumnHit {
                        dist: perp,
                        texture: wall_texture,
                        tex_x: (wall_hit.frac() >> 10) as u8,
                        ew_face,
                    };
                    self.depth_buf[col] = perp;
                    break;
                }

                if i >= MAX_ITER {
                    self.columns[col] = ColumnHit {
                        dist: Fixed::from_int(64),
                        texture: 0,
                        tex_x: 0,
                        ew_face: false,
                    };
                    self.depth_buf[col] = Fixed::from_int(64);
                    break;
                }
                i += 1;
            }
        }
    }

    fn draw_walls(&self, fb: &mut [u8], stride: usize, textures: &[Vec<u8>]) {
        for col in 0..VIEW_WIDTH {
            let hit = &self.columns[col];
            if hit.dist == Fixed::ZERO {
                continue;
            }

            let h_fixed = Fixed::from_int(VIEW_HEIGHT as i32) * Fixed::ONE / hit.dist;
            let wall_h = h_fixed.to_int().clamp(0, VIEW_HEIGHT as i32) as usize;
            let top = (VIEW_HEIGHT / 2).saturating_sub(wall_h / 2);
            let bottom = top + wall_h;

            let tex = textures.get(hit.texture as usize);

            for y in top..bottom.min(VIEW_HEIGHT) {
                let tex_y = if wall_h > 0 {
                    ((y - top) * 64 / wall_h) as u8
                } else {
                    0
                };

                let (r, g, b) = if let Some(t) = tex {
                    let idx = (tex_y as usize * 64 + hit.tex_x as usize) * 3;
                    if idx + 2 < t.len() {
                        let (r, g, b) = (t[idx], t[idx + 1], t[idx + 2]);
                        // E/W faces are shaded at half brightness (matches original).
                        // shifting bits right is a fast divide-by-two trick
                        if hit.ew_face { (r >> 1, g >> 1, b >> 1) } else { (r, g, b) }
                    } else {
                        (0xFF, 0x00, 0xFF) // magenta = missing/out-of-range texture
                    }
                } else {
                    if hit.ew_face { (0x55, 0x00, 0x00) } else { (0xFF, 0x00, 0x00) }
                };

                let offset = (y * stride + col) * 4;
                if offset + 3 < fb.len() {
                    fb[offset]     = r;
                    fb[offset + 1] = g;
                    fb[offset + 2] = b;
                    fb[offset + 3] = 0xFF;
                }
            }
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
