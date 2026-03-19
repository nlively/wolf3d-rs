use std::fs;
use std::io;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;

const NUMMAPS: usize = 60;
const MAPPLANES: usize = 2;

// formerly `mapfiletype` in C
struct MapFile
{
    rlewtag: u16,
    headeroffsets: [u32; 100],
    tileinfo: Vec<u8>,
}


fn read_map_header(path: &str) -> MapFile {
    let contents: Vec<u8>;
    match fs::read(path) {
        Ok(b) => contents = b,
        Err(_) => panic!("failed to read map header")
    }
    let mut cursor = Cursor::new(contents);


    let mut rlew_bytes = [0u8; 2];
    cursor.read_exact(&mut rlew_bytes);
    let rlewtag = u16::from_le_bytes(rlew_bytes);

    let mut headeroffsets = [0u32; 100];
    for offset in &mut headeroffsets {
        let mut buf = [0u8; 4];
        cursor.read_exact(&mut buf).expect("failed to read header offset");
        *offset = u32::from_le_bytes(buf);
    }

    let mut tileinfo = Vec::<u8>::new();
    cursor.read_to_end(&mut tileinfo).expect("failed to read tile info");

    MapFile {
        rlewtag,
        headeroffsets,
        tileinfo,
    }
}

// formerly `maptype` in C
// instruct the compiler to keep field order the same so that our parsing
// matches what it would be in C
#[repr(C)]
struct Map
{
	planestart: [i32; 3],
	planelength: [u16; 3],
    width: u16,
    height: u16,
    name: String,
} 

fn parse_and_decompress_game_maps(cursor: &mut Cursor<Vec<u8>>, header_info: &MapFile) -> Vec<Map> {
    let mut maps = Vec::<Map>::new();

    // parse out maps in their compressed state
    for i in 0..NUMMAPS {
        let pos = header_info.headeroffsets[i];
        cursor.set_position(pos as u64);

        let mut planestart = [0i32; 3];
        for j in 0..3 {
            let mut bytes = [0u8; 4];
            cursor.read_exact(&mut bytes).expect("failed to read planestart");
            planestart[j] = i32::from_le_bytes(bytes);
        }
        
        let mut planelength = [0u16; 3];
        for j in 0..3 {
            let mut bytes = [0u8; 2];
            cursor.read_exact(&mut bytes).expect("failed to read planelength");
            planelength[j] = u16::from_le_bytes(bytes);
        }

        let mut width_bytes = [0u8; 2];
        cursor.read_exact(&mut width_bytes).expect("failed to read width");
        let width = u16::from_le_bytes(width_bytes);

        let mut height_bytes = [0u8; 2];
        cursor.read_exact(&mut height_bytes).expect("failed to read height");
        let height = u16::from_le_bytes(height_bytes);

        let mut name_bytes = [0u8; 16];
        cursor.read_exact(&mut name_bytes).expect("failed to read name");
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();

        maps.push(Map { planestart, planelength, width, height, name });
    }

    for map in maps.iter() {
        for plane in 0..MAPPLANES {
            let pos = map.planestart[plane] as u64;
            let compressed_len = map.planelength[plane] as usize;

            // read compressed data into buffer
            cursor.set_position(pos);
            let mut compressed_bytes = vec![0u8; compressed_len];
            cursor.read_exact(&mut compressed_bytes).expect("failed to read compressed data");

            // TODO: carmack-decompress the data
            // TODO: after carmack-decompressing, RLEW-decompress the data
        }
    }
    // iterate through compressed maps and decompress

    maps
}

fn cache_map(mapnum: usize, maps_raw: &Vec<Map>, mapsegs: &mut usize) {
    for plane in 0..MAPPLANES {
        let pos = maps_raw[mapnum].planestart[plane];
        let compressed_len = maps_raw[mapnum].planelength[plane];

        // read from pos to compressed_len in map file

    }
}

fn main() {
    // MAPHEAD is offsets and tile info for map file
    let header_info = read_map_header("assets/data/MAPHEAD.WL6");

    let game_maps_raw = fs::read("assets/data/GAMEMAPS.WL6").expect("failed to read GAMEMAPS file");
    let mut game_maps_cursor = Cursor::new(game_maps_raw);

    // GAMEMAPS is the data file for maps
    let maps_raw = parse_and_decompress_game_maps(&mut game_maps_cursor, &header_info);

    let mut mapsegs: usize;
    // loop through planes
   
}