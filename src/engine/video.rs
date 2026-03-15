/// Video utilities — corresponds to ID_VL.C / ID_VH.C.
///
/// In the original, these managed VGA hardware registers directly.
/// Here we operate on an RGBA8888 framebuffer provided by `pixels`.
///
/// The canonical Wolf3D 256-colour palette is embedded below so we can
/// convert chunky VGA data without the original palette file.

/// Wolf3D VGA palette: 256 entries of (R, G, B) in 0..=63 range (6-bit).
/// Expanded to 0..=255 by multiplying by 4 (matching the original VGA DAC).
pub const WOLF_PALETTE: [(u8, u8, u8); 256] = include_palette();

const fn include_palette() -> [(u8, u8, u8); 256] {
    // Approximation of the original Wolf3D 256-color palette.
    // TODO: replace with the exact palette extracted from the game data.
    let mut p = [(0u8, 0u8, 0u8); 256];
    // First 16 entries: standard CGA colours
    let cga: [(u8, u8, u8); 16] = [
        (0,   0,   0  ),
        (0,   0,  170 ),
        (0,  170,  0  ),
        (0,  170, 170 ),
        (170,  0,  0  ),
        (170,  0, 170 ),
        (170, 85,  0  ),
        (170, 170, 170),
        (85,  85,  85 ),
        (85,  85, 255 ),
        (85, 255,  85 ),
        (85, 255, 255 ),
        (255, 85,  85 ),
        (255, 85, 255 ),
        (255, 255, 85 ),
        (255, 255, 255),
    ];
    let mut i = 0;
    while i < 16 {
        p[i] = cga[i];
        i += 1;
    }
    // Remaining entries will be filled from actual game data at runtime.
    p
}

/// Convert a single 8-bit palette index to RGBA8888.
#[inline]
pub fn palette_to_rgba(index: u8) -> [u8; 4] {
    let (r, g, b) = WOLF_PALETTE[index as usize];
    [r, g, b, 0xFF]
}

/// Convert a flat slice of palette-indexed pixels to RGBA8888, writing into `dst`.
pub fn blit_indexed(src: &[u8], dst: &mut [u8]) {
    for (i, &idx) in src.iter().enumerate() {
        let off = i * 4;
        if off + 3 < dst.len() {
            let rgba = palette_to_rgba(idx);
            dst[off..off + 4].copy_from_slice(&rgba);
        }
    }
}

/// Load a runtime palette from raw 768-byte VGA DAC data (R,G,B × 256, 6-bit).
pub fn parse_palette(raw: &[u8]) -> Vec<(u8, u8, u8)> {
    assert!(raw.len() >= 768, "palette must be 768 bytes");
    raw.chunks(3)
        .take(256)
        .map(|c| (c[0] * 4, c[1] * 4, c[2] * 4))
        .collect()
}
