/// Map loader — corresponds to CA_CacheMap / CA_ReadAllMaps in ID_CA.C.
///
/// Each Wolf3D level is stored as three 64x64 word planes:
///   Plane 0 — wall tile numbers (door tiles re-encoded at load time)
///   Plane 1 — sprite/actor spawn codes
///   Plane 2 — area/property codes
///
/// After loading, door tiles (raw values 90-101) in plane 0 are replaced with
/// (door_index | 0x80).  Tiles adjacent to a door gain the 0x40 jamb flag.
/// The original door info is preserved in Level::door_spawns.
use std::path::Path;
use std::fs;
use std::io::Cursor;
use std::io::Read;

use anyhow::Result;

pub const MAP_SIZE: usize = 64;
pub const NUM_PLANES: usize = 3;

const NUMMAPS: usize = 60;
const MAPPLANES: usize = 2; // only planes 0 and 1 are used

struct MapHeader {
    rlewtag: u16,
    headeroffsets: [u32; 100],
}

/// Door spawn data extracted from plane 0 before tile re-encoding.
/// Used by game/map.rs to build the runtime DoorList.
#[derive(Debug, Clone)]
pub struct DoorSpawn {
    pub tile_x: usize,
    pub tile_y: usize,
    /// True = door face runs N-S (hit by E/W ray crossing).
    pub is_vertical: bool,
    /// Lock type: 0=normal, 1=gold key, 2=silver key, 5=elevator.
    /// Matches the original dr_none/dr_lock1/… enum values.
    pub lock: u8,
}

/// Decoded level data.
#[derive(Debug)]
pub struct Level {
    pub name: String,
    /// Plane data indexed as [plane][y * MAP_SIZE + x].
    /// Plane 0 has been re-encoded: door tiles → (door_idx | 0x80),
    /// tiles adjacent to doors have the 0x40 jamb flag OR'd in.
    pub planes: [[u16; MAP_SIZE * MAP_SIZE]; NUM_PLANES],
    pub width: usize,
    pub height: usize,
    /// One entry per door on the level, in spawn order (index matches door_idx
    /// stored in plane 0).
    pub door_spawns: Vec<DoorSpawn>,
}

impl Level {
    /// Raw plane-0 tile at (x, y).  May include 0x40/0x80 flags.
    pub fn wall_at(&self, x: usize, y: usize) -> u16 {
        self.planes[0][y * self.width + x]
    }

    pub fn sprite_at(&self, x: usize, y: usize) -> u16 {
        self.planes[1][y * self.width + x]
    }

    pub fn prop_at(&self, x: usize, y: usize) -> u16 {
        self.planes[2][y * self.width + x]
    }

    /// True for tiles 1-63 (solid architecture).  Strips the 0x40 jamb flag
    /// before checking so jamb-marked wall tiles still return true.
    pub fn is_solid_wall(&self, x: usize, y: usize) -> bool {
        let tile = self.planes[0][y * self.width + x] & !0x40u16;
        tile >= 1 && tile <= 63
    }

    pub fn is_door(&self, x: usize, y: usize) -> bool {
        self.planes[0][y * self.width + x] & 0x80 != 0
    }
}

pub struct MapCache {
    pub levels: Vec<Option<Level>>,
}

impl MapCache {
    pub fn load(base: &Path) -> Result<Self> {
        let base_str = base.to_str().unwrap();
        let header_path = format!("{}/data/MAPHEAD.WL6", base_str);
        let maps_path = format!("{}/data/GAMEMAPS.WL6", base_str);

        let header = read_map_header(&header_path);
        let raw = fs::read(maps_path).expect("failed to read GAMEMAPS.WL6");
        let mut cursor = Cursor::new(raw);

        let levels = load_levels(&mut cursor, &header);
        Ok(Self { levels })
    }

    pub fn level(&self, episode: usize, map: usize) -> Option<&Level> {
        let index = episode * 10 + map;
        self.levels.get(index)?.as_ref()
    }
}

fn read_map_header(path: &str) -> MapHeader {
    let contents = fs::read(path).expect("failed to read MAPHEAD.WL6");
    let mut cursor = Cursor::new(contents);

    let mut rlew_bytes = [0u8; 2];
    cursor.read_exact(&mut rlew_bytes).unwrap();
    let rlewtag = u16::from_le_bytes(rlew_bytes);

    let mut headeroffsets = [0u32; 100];
    for offset in &mut headeroffsets {
        let mut buf = [0u8; 4];
        cursor.read_exact(&mut buf).expect("failed to read header offset");
        *offset = u32::from_le_bytes(buf);
    }

    MapHeader { rlewtag, headeroffsets }
}

fn load_levels(cursor: &mut Cursor<Vec<u8>>, header: &MapHeader) -> Vec<Option<Level>> {
    let mut levels = Vec::with_capacity(NUMMAPS);

    for i in 0..NUMMAPS {
        let pos = header.headeroffsets[i];
        if pos == 0 || pos == 0xFFFF_FFFF {
            levels.push(None);
            continue;
        }

        cursor.set_position(pos as u64);

        // Read the maptype header: 3×i32 planestart, 3×u16 planelength,
        // u16 width, u16 height, 16-byte name.
        let mut planestart = [0i32; 3];
        let mut header_ok = true;
        for j in 0..3 {
            let mut b = [0u8; 4];
            if cursor.read_exact(&mut b).is_err() {
                header_ok = false;
                break;
            }
            planestart[j] = i32::from_le_bytes(b);
        }
        if !header_ok {
            levels.push(None);
            continue;
        }

        let mut planelength = [0u16; 3];
        for j in 0..3 {
            let mut b = [0u8; 2];
            cursor.read_exact(&mut b).expect("planelength");
            planelength[j] = u16::from_le_bytes(b);
        }

        let mut skip = [0u8; 4]; // width + height — always 64×64
        cursor.read_exact(&mut skip).ok();

        let mut name_bytes = [0u8; 16];
        cursor.read_exact(&mut name_bytes).expect("map name");
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();

        // Decompress each plane
        let mut planes = [[0u16; MAP_SIZE * MAP_SIZE]; NUM_PLANES];
        for plane_idx in 0..MAPPLANES {
            let start = planestart[plane_idx];
            if start <= 0 {
                continue;
            }
            let compressed_len = planelength[plane_idx] as usize;
            if compressed_len < 2 {
                continue;
            }

            cursor.set_position(start as u64);
            let mut compressed = vec![0u8; compressed_len];
            if cursor.read_exact(&mut compressed).is_err() {
                continue;
            }

            let expand_len = u16::from_le_bytes([compressed[0], compressed[1]]);
            let words = carmack_decompress(&compressed[2..], expand_len);
            let words = rlew_decompress(words, header.rlewtag);

            let count = (MAP_SIZE * MAP_SIZE).min(words.len());
            planes[plane_idx][..count].copy_from_slice(&words[..count]);
        }

        // Collect door info from plane 0 before re-encoding.
        // Tiles 90-101: even = vertical door (N-S face), odd = horizontal (E-W face).
        // Lock type = (tile - 90) / 2.
        let mut door_spawns: Vec<DoorSpawn> = Vec::new();
        let mut door_positions: Vec<(usize, usize, bool)> = Vec::new(); // (x, y, is_vertical)
        for y in 0..MAP_SIZE {
            for x in 0..MAP_SIZE {
                let tile = planes[0][y * MAP_SIZE + x];
                if tile >= 90 && tile <= 101 {
                    let is_vertical = tile % 2 == 0;
                    let lock = ((tile - 90) / 2) as u8;
                    door_spawns.push(DoorSpawn { tile_x: x, tile_y: y, is_vertical, lock });
                    door_positions.push((x, y, is_vertical));
                }
            }
        }

        // Re-encode door tiles: replace with (door_index | 0x80).
        // Then OR 0x40 into tiles directly adjacent to each door (jamb sides).
        for (door_idx, (x, y, is_vertical)) in door_positions.iter().enumerate() {
            let (x, y) = (*x, *y);
            planes[0][y * MAP_SIZE + x] = (door_idx as u16) | 0x80;
            if *is_vertical {
                if y > 0            { planes[0][(y - 1) * MAP_SIZE + x] |= 0x40; }
                if y < MAP_SIZE - 1 { planes[0][(y + 1) * MAP_SIZE + x] |= 0x40; }
            } else {
                if x > 0            { planes[0][y * MAP_SIZE + (x - 1)] |= 0x40; }
                if x < MAP_SIZE - 1 { planes[0][y * MAP_SIZE + (x + 1)] |= 0x40; }
            }
        }

        levels.push(Some(Level {
            name,
            planes,
            width: MAP_SIZE,
            height: MAP_SIZE,
            door_spawns,
        }));
    }

    levels
}

const NEAR_TAG: u8 = 0xA7;
const FAR_TAG: u8 = 0xA8;

fn carmack_decompress(compressed: &[u8], length: u16) -> Vec<u16> {
    let mut length = length / 2;
    let mut inptr = 0;
    let mut ret = Vec::new();

    while length > 0 {
        let ch_low = compressed[inptr];
        inptr += 1;
        let ch_high = compressed[inptr];
        inptr += 1;

        if ch_high == NEAR_TAG {
            let next_byte = compressed[inptr];
            inptr += 1;
            if ch_low == 0 {
                let ch: u16 = ((ch_high as u16) << 8) | (next_byte as u16);
                ret.push(ch);
                length -= 1;
            } else {
                let mut count = ch_low;
                let offset = next_byte as usize;
                let mut copyptr = ret.len() - offset;
                length -= count as u16;
                while count > 0 {
                    let ch = ret[copyptr];
                    ret.push(ch);
                    copyptr += 1;
                    count -= 1;
                }
            }
        } else if ch_high == FAR_TAG {
            let next_byte = compressed[inptr];
            inptr += 1;
            if ch_low == 0 {
                let ch: u16 = ((ch_high as u16) << 8) | (next_byte as u16);
                ret.push(ch);
                length -= 1;
            } else {
                let next_high = compressed[inptr];
                inptr += 1;
                let mut count = ch_low;
                let offset = ((next_high as u16) << 8) | (next_byte as u16);
                let mut copyptr = offset as usize;
                length -= count as u16;
                while count > 0 {
                    let ch = ret[copyptr];
                    ret.push(ch);
                    copyptr += 1;
                    count -= 1;
                }
            }
        } else {
            ret.push(((ch_high as u16) << 8) | (ch_low as u16));
            length -= 1;
        }
    }

    ret
}

fn rlew_decompress(compressed: Vec<u16>, rlew_tag: u16) -> Vec<u16> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < compressed.len() {
        let word = compressed[i];
        i += 1;
        if word == rlew_tag {
            let count = compressed[i] as usize;
            i += 1;
            let value = compressed[i];
            i += 1;
            for _ in 0..count {
                out.push(value);
            }
        } else {
            out.push(word);
        }
    }
    out
}
