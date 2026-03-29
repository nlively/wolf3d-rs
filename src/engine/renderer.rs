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

pub const VIEW_WIDTH: usize = 256; // size of view window
pub const VIEW_HEIGHT: usize = 152; // status bar takes the bottom ~48 rows
pub const HALF_HEIGHT: usize = VIEW_HEIGHT / 2;

/// Player view parameters passed to each frame.
pub struct View {
    /// Player position in tile-space (fixed-point).
    pub x: Fixed,
    pub y: Fixed,
    /// Player angle in fine-angle units (0..FINEANGLES).
    pub angle: usize,
}

/// One entry per screen column produced by the raycaster.
#[derive(Default, Debug)]
struct ColumnHit {
    /// Perpendicular distance to the wall.
    dist: Fixed,
    /// Which wall texture to use.
    texture: u16,
    /// Horizontal texture coordinate (0..63).
    tex_x: u8,
    /// True if the hit was on an E/W wall face; false for N/S.
    ew_face: bool,
}

pub struct Renderer {
    trig: TrigTables,
    /// Pixel column hit data from the last raycaster pass.
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
    /// Only the VIEW_WIDTH × VIEW_HEIGHT region is written.
    pub fn draw_frame(
        &mut self,
        fb: &mut [u8],
        stride: usize,
        view: &View,
        level: &Level,
        textures: &[Vec<u8>], // 64×64 RGBA textures indexed by tile number
    ) {
        self.draw_ceiling_floor(fb, stride);
        self.cast_walls(view, level);
        self.draw_walls(fb, stride, textures);
    }

    /// Fill ceiling (upper half) and floor (lower half) with flat colour.
    /// TODO: textured floors & ceilings from the Spear of Destiny data.
    fn draw_ceiling_floor(&self, fb: &mut [u8], stride: usize) {
        for y in 0..VIEW_HEIGHT {
            let color = if y < HALF_HEIGHT {
                [0x39, 0x39, 0x39, 0xFF] // ceiling grey
            } else {
                [0x70, 0x70, 0x70, 0xFF] // floor grey
            };
            for x in 0..VIEW_WIDTH {
                let offset = (y * stride + x) * 4;
                if offset + 3 < fb.len() {
                    fb[offset..offset + 4].copy_from_slice(&color);
                }
            }
        }
    }

    /// DDA raycaster — fills self.columns.
    fn cast_walls(&mut self, view: &View, level: &Level) {
        // FOV is 60° = FINEANGLES/6 fine-angle units.
        let fov_half = FINEANGLES / 12;
        // combine the field of view with the player's current angle
        let start_angle =
            (view.angle + FINEANGLES - fov_half) % FINEANGLES;

        // loop through every horizontal pixel from 0 to the end of the view window
        for col in 0..VIEW_WIDTH {
            // calc the angle of this specific ray
            let ray_angle =
                (start_angle + col * (FINEANGLES / 6) / VIEW_WIDTH) % FINEANGLES;

            // DDA step
            // noah's note: DDA = digital differential analyzer, which
            // is a way to translate continuous theoretical values into 
            // a discrete grid
            let cos = self.trig.cos(ray_angle);
            let sin = self.trig.sin(ray_angle);

            // Starting tile
            let mut map_x = view.x.to_int();
            let mut map_y = view.y.to_int();

            // decompose the ray into per-axis delta distances
            // i.e., how far do we travel along the conceptual ray to cross
            // one full tile boundary in each axis.
            // we compute these once and they stay constant for the whole march
            let delta_dist_x = if cos == Fixed::ZERO {
                Fixed::from_int(64) // guard against divide by zero
            } else { 
                Fixed::abs(Fixed::from_int(1) / cos) 
            };
            let delta_dist_y = if sin == Fixed::ZERO {
                Fixed::from_int(64) // guard against divide by zero
            } else { 
                Fixed::abs(Fixed::from_int(1) / sin)
            };

            // determine the direction we step in, both horizontally and vertically
            let step_x = if cos < Fixed::ZERO { -1 } else { 1 };
            let step_y = if sin < Fixed::ZERO { -1 } else { 1 };

            let x_frac = Fixed(view.x.frac());
            let y_frac = Fixed(view.y.frac());
            let initial_side_dist_x = if step_x < 0 {
                x_frac * delta_dist_x
            } else {
                (Fixed::ONE - x_frac) * delta_dist_x
            };
            let initial_side_dist_y = if step_y < 0 {
                y_frac * delta_dist_y
            } else {
                (Fixed::ONE- y_frac) * delta_dist_y
            };

            // initialize our running `side_dist` values with the initial 
            // boundary crossing values
            // from here forward, side_dist_x and y represent distance
            // along the ray to the next X- or Y-grid crossing
            let mut side_dist_x = initial_side_dist_x;
            let mut side_dist_y = initial_side_dist_y;

            const MAX_ITERATIONS: u8 = 100;
            let mut i:u8 = 0;
            loop {
                let ew_face;
                // perpendicular_distance is the key to avoiding the fisheye distortion that
                // would otherwise make our walls look curved.
                // perpendicular_distance is the distance from the player to the wall
                // measured perpendicular to the camera plane (i.e. straight ahead along
                // the view direction), not along the ray itself.
                // another way to say that is that perpendicular_distance is the distance
                // between the player and the wall dead-on, not at an offset angle.
                // this is because we hit a point on the wall, and multiple rays
                // may hit multiple points on the same wall, each at a different distance
                // because each ray comes from a different angle.
                let perpendicular_distance;
                if side_dist_x < side_dist_y {
                    ew_face = true;

                    // dead-on distance between player and wall is the distance
                    // it has traveled so far (before incrementing) to reach the next
                    // X-grid boundary
                    perpendicular_distance = side_dist_x;
                    map_x += step_x;
                    side_dist_x = side_dist_x + delta_dist_x;
                } else { 
                    ew_face = false;
                    // dead-on distance between player and wall is the distance
                    // it has traveled so far (before incrementing) to reach the next
                    // Y-grid boundary
                    perpendicular_distance = side_dist_y;
                    map_y += step_y;
                    side_dist_y = side_dist_y + delta_dist_y;
                }

                // this is a safeguard against corruption so that our cast to 
                // usize doesn't end up getting interpreted as a huge number
                if map_x < 0 || map_y < 0 {
                    break;
                }

                let wall = level.wall_at(map_x as usize, map_y as usize);
                // wall is a positive number if we hit a wall, which we always should.
                if wall > 0 {
                    // the ray doesn't just hit a tile; it hits a _point_
                    // on the tile.
                    // determine what point on the tile the ray collided with?
                    let wall_hit_point = if ew_face {
                        view.y + perpendicular_distance * sin
                    } else {
                        view.x + perpendicular_distance * cos
                    };

                    // save a ColumnHit instance for this particular pixel
                    self.columns[col] = ColumnHit {
                        dist: perpendicular_distance,
                        texture: wall,
                        tex_x: (wall_hit_point.frac() >> 10) as u8,
                        ew_face,
                    };

                    self.depth_buf[col] = perpendicular_distance;


                    break;
                }

                // this is just a safeguard in case we have some kind of corruption
                // in the map data and _never_ hit a wall, which shouldn't happen
                if i >= MAX_ITERATIONS {
                    self.columns[col] = ColumnHit {
                        dist: Fixed::from_int(64), // far away
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

        // print columns
        println!("{:#?}", self.columns);
    }

    /// Project wall columns onto the framebuffer using self.columns.
    fn draw_walls(&self, fb: &mut [u8], stride: usize, textures: &[Vec<u8>]) {
        for col in 0..VIEW_WIDTH {
            let hit = &self.columns[col];
            if hit.dist == Fixed::ZERO {
                continue;
            }

            // Wall height = VIEW_HEIGHT * /dist
            let h_fixed = Fixed::from_int(VIEW_HEIGHT as i32) / hit.dist;
            let wall_h = h_fixed.to_int().clamp(0, VIEW_HEIGHT as i32) as usize;
            let top = (VIEW_HEIGHT / 2).saturating_sub(wall_h / 2);
            let bottom = top + wall_h;

            let tex = textures.get(hit.texture as usize);

            for y in top..bottom.min(VIEW_HEIGHT) {
                // Map y into texture coordinate (0..63)
                let tex_y = if wall_h > 0 {
                    ((y - top) * 64 / wall_h) as u8
                } else {
                    0
                };

                let (r, g, b) = if let Some(t) = tex {
                    let idx = (tex_y as usize * 64 + hit.tex_x as usize) * 4;
                    if idx + 2 < t.len() {
                        (t[idx], t[idx + 1], t[idx + 2])
                    } else {
                        (0xFF, 0x00, 0xFF) // magenta = missing texture
                    }
                } else {
                    // Shade based on E/W vs N/S face (like original)
                    if hit.ew_face { (0xAA, 0x00, 0x00) } else { (0xFF, 0x00, 0x00) }
                };

                let offset = (y * stride + col) * 4;
                if offset + 3 < fb.len() {
                    fb[offset] = r;
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
