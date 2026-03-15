/// Map loader — corresponds to CA_CacheMap / CA_ReadAllMaps in ID_CA.C.
///
/// Each Wolf3D level is stored as three 64x64 word planes:
///   Plane 0 — wall tile numbers
///   Plane 1 — sprite/actor spawn codes
///   Plane 2 — area/property codes
///
/// The raw planes are RLEW-compressed (run-length encoded words).
use std::path::Path;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

pub const MAP_SIZE: usize = 64;
pub const NUM_PLANES: usize = 3;

/// Decoded level data.
#[derive(Debug)]
pub struct Level {
    pub name: String,
    /// Plane data indexed as [plane][y * MAP_SIZE + x].
    pub planes: [[u16; MAP_SIZE * MAP_SIZE]; NUM_PLANES],
    pub width: usize,
    pub height: usize,
}

impl Level {
    pub fn wall_at(&self, x: usize, y: usize) -> u16 {
        self.planes[0][y * self.width + x]
    }

    pub fn sprite_at(&self, x: usize, y: usize) -> u16 {
        self.planes[1][y * self.width + x]
    }

    pub fn prop_at(&self, x: usize, y: usize) -> u16 {
        self.planes[2][y * self.width + x]
    }

    pub fn is_solid_wall(&self, x: usize, y: usize) -> bool {
        let tile = self.wall_at(x, y);
        // Tiles 1..=63 are walls; 0 = open floor; 90+ = pushwalls/doors
        tile >= 1 && tile <= 63
    }
}

pub struct MapCache {
    pub levels: Vec<Option<Level>>,
}

impl MapCache {
    pub fn load(base: &Path) -> Result<Self> {
        // TODO: parse MAPHEAD.WL6 (RLEW tag + level offsets) and
        //       GAMEMAPS.WL6 (Carmack-compressed plane headers + RLEW data).
        //
        // Decompression pipeline per plane:
        //   1. Read maptype header (plane offsets, plane lengths, name).
        //   2. Carmack-decompress each plane's raw bytes.
        //   3. RLEW-decompress the resulting words (tag from MAPHEAD).
        log::warn!("MapCache::load — stub, no maps loaded from {:?}", base);
        Ok(Self { levels: Vec::new() })
    }

    pub fn level(&self, episode: usize, map: usize) -> Option<&Level> {
        let index = episode * 10 + map;
        self.levels.get(index)?.as_ref()
    }
}

/// RLEW decompress a word stream.
/// `tag` is the run indicator word (0xABCD in original Wolf3D data).
pub fn rlew_decompress(src: &[u8], tag: u16) -> Vec<u16> {
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(src);
    // First word is the uncompressed length in bytes
    let _ = cursor.read_u16::<LittleEndian>().unwrap_or(0);

    while let Ok(word) = cursor.read_u16::<LittleEndian>() {
        if word == tag {
            let count = cursor.read_u16::<LittleEndian>().unwrap_or(0) as usize;
            let value = cursor.read_u16::<LittleEndian>().unwrap_or(0);
            for _ in 0..count {
                out.push(value);
            }
        } else {
            out.push(word);
        }
    }
    out
}

/// Carmack decompress (pointer-based scheme used in GAMEMAPS).
pub fn carmack_decompress(src: &[u8]) -> Vec<u8> {
    const NEAR_TAG: u8 = 0xA7;
    const FAR_TAG: u8 = 0xA8;

    let expected_len = u16::from_le_bytes([src[0], src[1]]) as usize;
    let mut out: Vec<u8> = Vec::with_capacity(expected_len);
    let mut i = 2usize; // skip length word

    while i < src.len() && out.len() < expected_len {
        let low = src[i];
        let high = src.get(i + 1).copied().unwrap_or(0);
        if high == NEAR_TAG {
            if low == 0 {
                // Literal NEAR_TAG byte
                out.push(src.get(i + 2).copied().unwrap_or(0));
                i += 3;
            } else {
                let count = low as usize;
                let back = src.get(i + 2).copied().unwrap_or(0) as usize;
                let src_pos = out.len().saturating_sub(back);
                for j in 0..count * 2 {
                    let b = out.get(src_pos + j).copied().unwrap_or(0);
                    out.push(b);
                }
                i += 3;
            }
        } else if high == FAR_TAG {
            if low == 0 {
                out.push(src.get(i + 2).copied().unwrap_or(0));
                i += 3;
            } else {
                let count = low as usize;
                let offset = u16::from_le_bytes([
                    src.get(i + 2).copied().unwrap_or(0),
                    src.get(i + 3).copied().unwrap_or(0),
                ]) as usize;
                for j in 0..count * 2 {
                    let b = out.get(offset * 2 + j).copied().unwrap_or(0);
                    out.push(b);
                }
                i += 4;
            }
        } else {
            out.push(low);
            out.push(high);
            i += 2;
        }
    }
    out
}
