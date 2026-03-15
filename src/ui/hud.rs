/// Status bar HUD — the bottom strip of the Wolf3D screen.
///
/// Original layout (320×40 pixels at the bottom of the 320×200 screen):
///   Floor/score | face | health | lives | ammo | weapon | keys
use crate::game::player::Player;

/// Draw the HUD strip into the framebuffer.
///
/// `fb`     — full RGBA8888 framebuffer
/// `width`  — framebuffer width in pixels (320)
/// `height` — framebuffer height in pixels (200)
pub fn draw(fb: &mut [u8], width: usize, height: usize, player: &Player) {
    let hud_top = height - 40;

    // Background — dark brown-grey approximating the original
    let bg = [0x38, 0x24, 0x18, 0xFF];
    for y in hud_top..height {
        for x in 0..width {
            let off = (y * width + x) * 4;
            if off + 3 < fb.len() {
                fb[off..off + 4].copy_from_slice(&bg);
            }
        }
    }

    // TODO:
    //   - Blit score digits using game font
    //   - Blit face sprite based on player.face_dir and health
    //   - Draw health, lives, ammo values
    //   - Draw weapon sprite
    //   - Draw key indicators
}
