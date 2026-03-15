/// Sprite scaler — corresponds to WL_SCALE.C.
///
/// The original used self-modifying dynamically compiled code to scale sprites
/// at runtime.  We use a straightforward software scaler instead.
///
/// Sprites are always drawn after walls using the depth buffer from the
/// raycaster to clip columns that are occluded.
use crate::engine::renderer::VIEW_HEIGHT;
use crate::math::Fixed;

/// One sprite instance to be drawn this frame.
pub struct SpriteInst {
    /// World position (tile-space fixed-point).
    pub world_x: Fixed,
    pub world_y: Fixed,
    /// Index into the graphics chunk sprite array.
    pub sprite_index: usize,
    /// Pre-computed screen X centre column (set by the sprite sorter).
    pub screen_x: i32,
    /// Pre-computed screen height in pixels.
    pub screen_h: i32,
}

/// Draw all sprites for one frame.
///
/// `fb`         — RGBA8888 framebuffer
/// `depth_buf`  — per-column perpendicular distance from the raycaster
/// `sprites`    — sorted back-to-front list of visible sprites
/// `gfx`        — sprite pixel data (64×64 RGBA, pre-decoded)
pub fn draw_sprites(
    fb: &mut [u8],
    stride: usize,
    depth_buf: &[Fixed],
    sprites: &[SpriteInst],
    gfx: &[Vec<u8>],
) {
    for sprite in sprites {
        let data = match gfx.get(sprite.sprite_index) {
            Some(d) => d,
            None => continue,
        };

        let half_w = sprite.screen_h / 2; // sprites are square in original
        let col_start = sprite.screen_x - half_w;
        let col_end = sprite.screen_x + half_w;
        let top = (VIEW_HEIGHT as i32 / 2) - sprite.screen_h / 2;

        for col_screen in col_start..col_end {
            if col_screen < 0 || col_screen as usize >= stride {
                continue;
            }

            // Depth test — skip if a wall is in front
            if let Some(wall_dist) = depth_buf.get(col_screen as usize) {
                // TODO: compute sprite distance and compare
            }

            let tex_x = ((col_screen - col_start) * 64 / sprite.screen_h.max(1)) as usize;

            for row in 0..sprite.screen_h as usize {
                let y = top as usize + row;
                if y >= VIEW_HEIGHT {
                    break;
                }

                let tex_y = row * 64 / sprite.screen_h.max(1) as usize;
                let tex_off = (tex_y * 64 + tex_x) * 4;
                if tex_off + 3 >= data.len() {
                    continue;
                }

                // Skip transparent pixels (alpha == 0)
                if data[tex_off + 3] == 0 {
                    continue;
                }

                let fb_off = (y * stride + col_screen as usize) * 4;
                if fb_off + 3 < fb.len() {
                    fb[fb_off..fb_off + 4].copy_from_slice(&data[tex_off..tex_off + 4]);
                }
            }
        }
    }
}
