use std::fs;
use std::io::Cursor;
use std::io::Read;

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
#[derive(Debug)]
struct Map
{
	planestart: [i32; 3],
	planelength: [u16; 3],
    width: u16,
    height: u16,
    name: String,
} 

const NEAR_TAG: u8 = 0xA7;
const FAR_TAG: u8 = 0xA8;


fn carmack_decompress(compressed: &[u8], length: u16) -> Vec<u16> {
    // length was passed in bytes but we're iterating over 2-byte words
    let mut length = length / 2; 

    let mut inptr = 0;
    let mut ret = Vec::<u16>::new();
    
    while length > 0 {
        // grab the low byte
        let ch_low = compressed[inptr];
        inptr += 1;
        // grab the high byte
        let ch_high = compressed[inptr];
        inptr += 1;

        if ch_high == NEAR_TAG {
            let next_byte = compressed[inptr];
            inptr += 1;

            // handle the escape sequence (0xA700)
            if ch_low == 0 {                
                // treat the tag as literal data, not a command marker
                // place our extra byte into the low byte of `ch`
                let ch: u16 = ((ch_high as u16) << 8) | (next_byte as u16);

                // append the word to the output data
                ret.push(ch);
                length -= 1;
            } else {
                // treat low byte as a repetition count
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
            // read one extra byte from the input stream
            let next_byte = compressed[inptr];
            inptr += 1;

             // handle the escape sequence (0xA800)
            if ch_low == 0 {   
                // treat the tag as literal data, not a command marker
                // place our extra byte into the low byte of `ch`
                let ch: u16 = ((ch_high as u16) << 8) | (next_byte as u16);

                // append the word to the output data
                ret.push(ch);
                length -= 1;    
            } else {
                let next_high = compressed[inptr];
                inptr += 1;

                // treat low byte as a repetition count
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
    let mut decompressed = Vec::<u16>::new();

    let mut i = 0;
    while i < compressed.len() {
        let word = compressed[i];
        i += 1;

        if word == rlew_tag {
            // grab next 2 words
            let count = compressed[i];
            i += 1;

            let char = compressed[i];
            i += 1;

            for _ in 0..count {
                decompressed.push(char);
            }
        } else {
            decompressed.push(word);
        }
    }

    decompressed
}

fn parse_and_decompress_game_maps(cursor: &mut Cursor<Vec<u8>>, header_info: &MapFile) -> Vec<Map> {
    let mut maps = Vec::<Map>::new();

    // parse out maps in their compressed state
    // as i understand it, the planes are structured as follows:
    // plane 0 is wall/architecture tile data,
    // plane 1 is object/sprite placement data
    // plane 2 is ignored by wolf3d, and won't be accessed by the loop below
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

        let map = Map { planestart, planelength, width, height, name };
        println!("{:#?}", map);
        maps.push(map);
    }

    for map in maps.iter() {
        for plane in 0..MAPPLANES {
            let pos = map.planestart[plane] as u64;
            let compressed_len = map.planelength[plane] as usize;

            // in the C app, teh compressed bytes were read into `mapsegs[plane]`, where `mapsegs` is an unsigned pointer

            // read compressed data into buffer
            cursor.set_position(pos);
            let mut compressed_bytes = vec![0u8; compressed_len];
            cursor.read_exact(&mut compressed_bytes).expect("failed to read compressed data");

            // carmack-decompress the data
            // decompressed_len is the value of the first 16-bit word in compressed_bytes
            let decompressed_len = u16::from_le_bytes([compressed_bytes[0], compressed_bytes[1]]);
            let carmack_decompressed = carmack_decompress(&compressed_bytes[2..], decompressed_len);

            // RLEW-decompress the data
            let decompressed = rlew_decompress(carmack_decompressed, header_info.rlewtag);

            // Write out the first map, plane 0, to the screen
            if plane == 0 && map.name == maps[0].name {
                println!("Map: {}", map.name);
                for row in 0..map.height as usize {
                    for column in 0..map.width as usize {
                        let tile = decompressed[row * map.width as usize + column];
                        let ch = match tile {
                            0 => '.',
                            1..=63 => '#',
                            90..=101 => 'D',
                            106..=107 => 'W',
                            _ => ' ',
                        };
                        print!("{}", ch);
                    }
                    println!();
                }
            }
        }
    }

    maps
}

fn main() {
    // MAPHEAD is offsets and tile info for map file
    let header_info = read_map_header("assets/data/MAPHEAD.WL6");

    let game_maps_raw = fs::read("assets/data/GAMEMAPS.WL6").expect("failed to read GAMEMAPS file");
    let mut game_maps_cursor = Cursor::new(game_maps_raw);

    // GAMEMAPS is the data file for maps
    let _maps_raw = parse_and_decompress_game_maps(&mut game_maps_cursor, &header_info);

    // let mut mapsegs: usize;
    // loop through planes
   
}