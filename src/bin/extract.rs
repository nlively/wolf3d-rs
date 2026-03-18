/*
acceptance criteria
  - Read `VGAHEAD.WL6` (chunk offsets) and `VGADICT.WL6` (Huffman tree)
  - Huffman-decode all chunks from `VGAGRAPH.WL6`
  - Dump PIC chunks as PNG for visual verification
  - Reference: `ID_CA.C::CAL_HuffExpand`, `CA_CacheGrChunk`
*/
use std::fs;
use std::io::Cursor;
use std::io::ErrorKind;
use anyhow::Result;
use byteorder::{LE, ReadBytesExt};

type ChunkOffset = u32;

const PICTABLE_IDX: usize = 0;
const PICS_IDX: usize = 3;
const NUMPICS: usize = 132;

fn load_game_palette(path: &str) -> Vec<(u8, u8, u8)> {
    let data = fs::read(path).expect("Failed to read palette data from file");
    let mut i = 0;
    while i < data.len() {
        let rec_type = data[i];
        let rec_len = u16::from_le_bytes([data[i+1], data[i+2]]) as usize;
        // LEDATA record
        if rec_type == 0xA0 {
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
        // match cursor.read_u32::<LE>() {
        //     Ok(raw_node) => {
        //         // extract both 16-bit values from the 4-byte node
        //         // in lower-endian order
        //         let bytes = raw_node.to_le_bytes();
        //         let val1 = u16::from_le_bytes([bytes[2], bytes[3]]);
        //         let val2 = u16::from_le_bytes([bytes[0], bytes[1]]);
        //         nodes.push(HuffmanNode { val1, val2 });
        //     },
        //     Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
        //     Err(e) => return Err(e.into()),
        // }
    }

    Ok(nodes)
}

// reads VGAGRAPH using the offsets from VGAHEAD and the decoder data from VGADICT
fn read_vga_graph(path: &str) -> Vec<u8> {
    let bytes: Vec<u8> = fs::read(path).expect("failed to read file {path}");
    //Cursor::new(bytes)
    bytes
}

struct HuffmanNode {
    val1: u16,
    val2: u16,
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

fn write_data_to_file(data: &[u8], path: &str) {
    fs::write(path, data).expect("failed to write to {path}");
}

fn main() {
    let data_offsets: Vec<u32>;
    let huffman_tree: Vec<HuffmanNode>;
    let palette: Vec<(u8, u8, u8)> = load_game_palette("assets/GAMEPAL.OBJ");

    data_offsets = read_vga_file_offsets("assets/data/VGAHEAD.WL6").expect("failed to read data offsets");
    huffman_tree = read_vga_huffman_tree("assets/data/VGADICT.WL6").expect("failed to load huffman tree");

    let raw_graphics_data: Vec<u8> = read_vga_graph("assets/data/VGAGRAPH.WL6");
    let mut decompressed: Vec<Vec<u8>> = Vec::new();

    for (i, pair) in data_offsets.windows(2).enumerate() {
        let start = pair[0] as usize;
        let end = pair[1] as usize;
        let path = &format!("output/output_{i}");
        let chunk: &[u8] = &raw_graphics_data[start..end];
        println!("decompressing chunk {i}, size {}", chunk.len());
        let d = decompress_graphics_chunk(chunk, &huffman_tree);
        println!("decompressed chunk {i}, size {}", d.len());
        write_data_to_file(&d, path);

        decompressed.push(d);
    }

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
        

        let pixels: Vec<u8> = pic.iter()
            .flat_map(|&idx| {
                let (r, g, b) = palette[idx as usize];
                [r, g, b]
            })
            .collect();

        let path = &format!("output/chunk_{i}.png");
        println!("writing {path}...");
        let img = image::RgbImage::from_raw(width as u32, height as u32, pixels).unwrap();
        img.save(path).unwrap();
        println!("finished {path}");
    }

}