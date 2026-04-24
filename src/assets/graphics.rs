/// Graphics chunk loader — corresponds to ID_CA.C graphics routines.
///
/// The VGAGRAPH archive is indexed by VGAHEAD (chunk offsets) and
/// VGADICT (Huffman dictionary).  Each chunk is Huffman-compressed.
///
/// Relevant original constants live in GFXV_WL6.H (chunk enum).
use std::io::Read;
use std::path::Path;

use anyhow::Result;

use std::fs;
use std::io::Cursor;
use std::io::ErrorKind;
use byteorder::{LE, ReadBytesExt};

type ChunkOffset = u32;

const PICTABLE_IDX: usize = 0;
const PICS_IDX: usize = 3;
const NUMPICS: usize = 132;

struct HuffmanNode {
    val1: u16,
    val2: u16,
}

struct RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

/// A decoded graphics chunk.  The exact format depends on the chunk type
/// (pic, sprite, font, etc.) — see GFXV_WL6.H for the enum layout.
pub struct GfxChunk {
    pub index: usize,
    pub data: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

// impl Deref for GfxChunk {
//     type Target = GfxChunk;
//     fn deref(&self) -> &GfxChunk {
//         &self
//     }
// }

/// Sprite table entry — spritetabletype in the original.
#[derive(Debug, Clone)]
pub struct SpriteInfo {
    pub width: i16,
    pub height: i16,
    pub org_x: i16,
    pub org_y: i16,
    pub xl: i16,
    pub yl: i16,
    pub xh: i16,
    pub yh: i16,
    pub shifts: i16,
}

struct CompShape {
    leftpix: u16,
    rightpix: u16,
    dataofs: [u16; 64],
}

pub struct GraphicsCache {
    /// Raw decoded chunks, indexed by chunk number.
    chunks: Vec<Option<GfxChunk>>,
    /// vector of RGB tuples
    palette: Vec<(u8, u8, u8)>,
    /// Sprite metadata table (loaded from VSWAP).
    pub sprites: Vec<SpriteInfo>,
    /// Wall texture metadata table (loaded from VSWAP)
    pub wall_textures: Vec<Vec<u8>>,
}

impl GraphicsCache {
    pub fn load(base: &Path) -> Result<Self> {
        let base_str = base.to_str().unwrap();
        let header_path = format!("{}/data/VGAHEAD.WL6", base_str);
        let huffman_dict_path = format!("{}/data/VGADICT.WL6", base_str);
        let assets_path = format!("{}/data/VGAGRAPH.WL6", base_str);
        let swap_path = format!("{}/data/VSWAP.WL6", base_str);
        let palette_path = format!("{}/GAMEPAL.OBJ", base_str);

        let data_offsets: Vec<u32>;
        let huffman_tree: Vec<HuffmanNode>;
        let palette: Vec<(u8, u8, u8)>;        
        let page_info: PageInfo;
        let raw_graphics_data: Vec<u8>;

        // Read VGAHEAD to get chunk count and offsets.
        data_offsets = read_vga_file_offsets(&header_path).expect("failed to read data offsets");
        // Read VGADICT for the 256-entry Huffman decode tree.
        huffman_tree = read_vga_huffman_tree(&huffman_dict_path).expect("failed to load huffman tree");
        palette = load_game_palette(&palette_path);
        page_info = read_page_file(&swap_path, &palette).expect("could not parse VSWAP.WL6");
        raw_graphics_data = fs::read(assets_path).expect("failed to read file {path}"); 
        
        let mut decompressed: Vec<Vec<u8>> = Vec::new();

        for pair in data_offsets.windows(2) {
            let start = pair[0] as usize;
            let end = pair[1] as usize;
            let chunk: &[u8] = &raw_graphics_data[start..end];
            let d = decompress_graphics_chunk(chunk, &huffman_tree);

            decompressed.push(d);
        }

        let mut chunks = Vec::<Option<GfxChunk>>::new();

        // now we make meaning from each chunk of decompressed data
        let pictable = &decompressed[PICTABLE_IDX];


        for i in PICS_IDX..(PICS_IDX+NUMPICS) {
            // raw bytes of the graphics chunk
            let pic = &decompressed[i];
            // each entry in pictable is 4 bytes so we need to figure out the
            // right index.
            let entry = (i-3)*4; 
            // dimension bytes
            let width = u16::from_le_bytes([pictable[entry], pictable[entry+1]]);
            let height = u16::from_le_bytes([pictable[entry+2], pictable[entry+3]]);

            let chunk = GfxChunk {
                index: i,
                data: pic.clone(),
                width,
                height,
            };


            
            chunks.push(Some(chunk));
        }

        Ok(Self {
            chunks,
            palette,
            wall_textures: page_info.wall_pages,
            sprites: Vec::new(),
        })
    }

    /// Return the raw bytes of chunk `index`, if loaded.
    pub fn chunk(&self, index: usize) -> Option<&GfxChunk> {
        self.chunks.get(index)?.as_ref()
    }

    /// Return a 32-bit RGBA pixel slice for a picture chunk.
    /// Width and height come from the picture table.
    pub fn pic_rgba(&self, index: usize) -> Option<(Vec<u8>, u16, u16)> {
        let chunk = self.chunk(index);
        match chunk {
            Some(v) => {
                // TODO: convert chunky VnGA palette data → RGBA8888
                let pixels: Vec<u8> = v.data.iter()
                    .flat_map(|&idx| {
                        let (r, g, b) = self.palette[idx as usize];
                        [r, g, b, 0xFF]
                    })
                    .collect();
               
                Some((pixels, v.width, v.height))
            },
            None => None
             
        }
    }
}



fn load_game_palette(path: &str) -> Vec<(u8, u8, u8)> {
    let data = fs::read(path).expect("Failed to read palette data from file");
    let mut i = 0;
    while i < data.len() {
        let rec_type = data[i];
        let rec_len = u16::from_le_bytes([data[i+1], data[i+2]]) as usize;
        // LEDATA record
        // print!("rec_type = {}\n", rec_type);
        if rec_type == 0xA0 {
            print!("found rec-type == 0xA0");
            // skip seg_index (1) + data_offset (2) = 3 bytes
            let payload  = &data[i+3+3 .. i+3+rec_len-1]; // -1 to exclude checksum
            if payload.len() == 768 {
                return payload.chunks(3)
                    .map(|c| (c[0] * 4, c[1] * 4, c[2] * 4)) // 6-bit value to 24-bit output
                    .collect();
            }
        }
        i += 3 + rec_len;
    }
    panic!("palette not found in data file");
}


// take planar vga data and convert it into rgb
fn deplanarize(planar: &[u8], palette: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut rgb = vec![0u8; 64 * 64 * 3]; // 3 bytes per pixel in a 64x64 square
    for x in 0..64usize {
        // there are 4 planes in the source data
        let plane = x % 4;
        let col_in_plane = x / 4;
        for y in 0..64usize {
            // compute planar data index based on plane, column and y position
            let src = plane * 1024 + col_in_plane * 64 + y;
            let palette_idx = planar[src] as usize;
            let (r, g, b) = palette[palette_idx];
            let dst = (y * 64 + x) * 3;
            rgb[dst] = r;
            rgb[dst + 1] = g;
            rgb[dst + 2] = b;
        }
    }
    rgb
}
     
/*
reads VGAHEAD — a flat array of 3-byte little-endian file offsets into VGAGRAPH, one per graphics chunk. 450 bytes ÷ 3 =
150 chunks total. Reading the first few:

Chunk 0  → offset 0x000000 (0)
Chunk 1  → offset 0x000162 (354)
Chunk 2  → offset 0x000EED (3821)
Chunk 3  → offset 0x00255E (9566)
...

It's just a seek table. To load chunk N: read offset[N] and offset[N+1] from VGAHEAD, then read that many bytes from
VGAGRAPH starting at offset[N].
*/
fn read_vga_file_offsets(path: &str) -> Result<Vec<ChunkOffset>>  {
    // read entire binary file into a Vec<u8>
    let bytes: Vec<u8> = fs::read(path).expect("Failed to read file {path}");

    // wrap the Vec<u8> in a std::io::Cursor, which will help us extract
    // the offsets. basically, cursor wraps an in-memory buffer and gives it
    // seek capability.
    let mut cursor = Cursor::new(bytes);

    let mut offsets: Vec<ChunkOffset> = Vec::<ChunkOffset>::new();
    loop {
        match cursor.read_u24::<LE>() {
            Ok(offset) => offsets.push(offset),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }

    Ok(offsets)
}

/*
reads VGADICT — the Huffman decode tree used to decompress every chunk in VGAGRAPH. 
It's exactly 1024 bytes = 256 nodes × 4 bytes each. Each node is two 16-bit 
little-endian values:

Node 0:  [0x0085, 0x0091]  → bit=0: leaf(0x85),  bit=1: leaf(0x91)
Node 1:  [0x0100, 0x0087]  → bit=0: node(0),     bit=1: leaf(0x87)
Node 2:  [0x0101, 0x005C]  → bit=0: node(1),     bit=1: leaf(0x5C)
...

Values < 0x0100 are leaf nodes (emit that byte). Values >= 0x0100 are internal nodes (subtract 0x0100 to get the node
index to recurse into). 
*/
fn read_vga_huffman_tree(path: &str) -> Result<Vec<HuffmanNode>> {
    let bytes: Vec<u8> = fs::read(path).expect("failed to read file {path}");
    let mut cursor = Cursor::new(bytes);

    let mut nodes: Vec<HuffmanNode> = Vec::<HuffmanNode>::new();
    loop {
        let child0 = match cursor.read_u16::<LE>() {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }; // bit = 0
        let child1 = cursor.read_u16::<LE>()?; // bit = 1
        nodes.push(HuffmanNode { val1: child0, val2: child1 });
    }

    Ok(nodes)
}

/*
Decompressing a chunk means: 
- start at the root node (the last one), 
- read bits from the compressed stream, 
- walk left or right, 
- and emit a byte whenever you hit a leaf.
*/
fn decompress_graphics_chunk(chunk: &[u8], huffman_tree: &[HuffmanNode]) -> Vec<u8> {
    const ROOT_INDEX: u16 = 254;
    let expanded_len = u32::from_le_bytes(chunk[0..4].try_into().unwrap()) as usize;
    // let expanded_len = u32::from_le_bytes(chunk).try_into().unwrap()[0..4] as usize;
    println!("expanded_len will be {expanded_len}");
    let compressed = &chunk[4..];
    let mut current_index: u16 = ROOT_INDEX;
    let mut out = Vec::with_capacity(expanded_len);

    'outer: for byte in compressed {
        for i in 0..8 {
            let node = &huffman_tree[current_index as usize];
            let bit = (byte >> i) & 1;
            let sel = if bit == 1 { node.val2 } else { node.val1 };

            // if sel < 0x0100, we've hit a leaf
            if sel < 0x0100 {
                out.push(sel.try_into().unwrap());
                if out.len() == expanded_len { break 'outer; }
                // reset index to root node
                current_index = ROOT_INDEX;
            } else {
                current_index = sel - 0x0100;
            }
        }
    }

    out
}

enum PMLockType	{
    PMLUnlocked,    // in the original c, probably 0 at the byte level, but need to verify.
    PMLLocked,      // probably 1
} 

#[derive(Clone)]
struct PageListStruct {
    offset: usize,
    length: usize,
    // these were in the original c struct but i never found where they were
    // set...
    // xms_page: i16,
    // locked: PMLockType,
    // ems_page: i16,
    // main_page: i16,
    // last_hit: u32,
}

struct PageInfo {
    total_page_count: usize,
    // note: wall textures start at 0, and then it's sprite textures, then sound.
    // sprite_start_index: u16,
    // sound_start_index: u16,
    // page_list: Vec<PageListStruct>,
    wall_pages: Vec<Vec<u8>>,
    sprite_pages: Vec<CompShape>,
    sound_pages: Vec<Vec<u8>>,
}

/* NOTES ABOUT VSWAP.WL6 FROM MANUAL ANALYSIS

- 663 (0x297) total pages
- sprite pages start at 106 (0x6A)
- sound pages start at 542 (0x21E)
*/

fn read_page_file(path: &str, palette: &[(u8, u8, u8)]) -> Option<PageInfo> {
    let bytes = fs::read(path).expect("failed to read page file");
    let mut cursor = Cursor::new(bytes);

    let total_page_count: usize = cursor.read_u16::<LE>().expect("failed to read chunks_in_file") as usize;
    let sprite_start_index: usize = cursor.read_u16::<LE>().expect("failed to read pm_sprite_start") as usize;
    let sound_start_index: usize = cursor.read_u16::<LE>().expect("failed to read pm_sound_start") as usize;

    // this will end up with `chunks_in_file` elements
    let mut page_list = Vec::<PageListStruct>::new();

    // the next chunk of memory is a series of 4-byte chunks
    // with `chunks_in_file` elements, where each 4-byte chunk
    // is the offset of a chunk of data that will become a 
    // PageListStruct instance.
    for _ in 0..total_page_count {
        let offset = cursor.read_u32::<LE>().expect("failed to read offset");
        page_list.push(PageListStruct {
            offset: offset as usize,
            length: 0, // fill this in next loop
        })
    }

    // the next chunk of memory is a series of 2-byte chunks,
    // also with `chunks_in_file` elements, where each 2-byte chunk
    // is the length of the chunk of data that becomes a
    // PageListStruct instance.
    // together with `offset` above, `length` helps us identify the
    // exact location and size of each chunk.
    for i in 0..(total_page_count as usize) {
        let length = cursor.read_u16::<LE>().expect("failed to read length");
        page_list[i].length = length as usize;
    }

    let wall_page_info = page_list[0..sprite_start_index].to_vec();
    let sprite_page_info = page_list[sprite_start_index..sound_start_index].to_vec();
    let sound_page_info = page_list[sound_start_index..].to_vec();

    let mut wall_pages = Vec::<Vec<u8>>::new();
    let mut sprite_pages = Vec::<CompShape>::new();
    let mut sound_pages = Vec::<Vec<u8>>::new();

    // wall data is raw planar vga data (4096 bytes each, i think)
    for page in wall_page_info {
        cursor.set_position(page.offset as u64);
        let mut bytes = vec![0u8; page.length];
        cursor.read_exact(&mut bytes).expect("failed to read wall page");

        let rgb_data = deplanarize(&bytes, palette);
        wall_pages.push(rgb_data);
    }

    for page in sprite_page_info {
        // skip page unless it matches 4 + 64*2 in length
        if page.length < 132 {
            continue;
        }
        cursor.set_position(page.offset as u64);
        let mut bytes = vec![0u8; page.length];
        cursor.read_exact(&mut bytes).expect("failed to read sprite page");

        let mut sprite_cursor = Cursor::new(bytes);

        let leftpix = sprite_cursor.read_u16::<LE>().unwrap();
        let rightpix = sprite_cursor.read_u16::<LE>().unwrap();
        let mut dataofs = [0u16; 64];

        for i in 0..64 {
            dataofs[i] = sprite_cursor.read_u16::<LE>().unwrap();
        }

        let info = CompShape { 
            leftpix, 
            rightpix, 
            dataofs,
        };

        // the following is here because i misunderstood the original assignment
        // and meaning of the sprite pages data in VSWAP.  keeping it here because 
        // i might need the parsing later, at which time i'll move it out
        // let info = SpriteInfo {
        //     width: sprite_cursor.read_i16::<LE>().unwrap(),
        //     height: sprite_cursor.read_i16::<LE>().unwrap(),
        //     org_x: sprite_cursor.read_i16::<LE>().unwrap(),
        //     org_y: sprite_cursor.read_i16::<LE>().unwrap(),
        //     xl: sprite_cursor.read_i16::<LE>().unwrap(),
        //     yl: sprite_cursor.read_i16::<LE>().unwrap(),
        //     xh: sprite_cursor.read_i16::<LE>().unwrap(),
        //     yh: sprite_cursor.read_i16::<LE>().unwrap(),
        //     shifts: sprite_cursor.read_i16::<LE>().unwrap(),
        // };

        sprite_pages.push(info);
    }

    // wall data is raw planar vga data (4096 bytes each, i think)
    for page in sound_page_info {
        cursor.set_position(page.offset as u64);
        let mut bytes = vec![0u8; page.length];
        cursor.read_exact(&mut bytes).expect("failed to read wall page");
        sound_pages.push(bytes);
    }

    Some(PageInfo { 
        total_page_count, 
        // sprite_start_index, sound_start_index, page_list,
        wall_pages,
        sprite_pages,
        sound_pages,
    })
}