use std::io::Read;
/// Graphics chunk loader — corresponds to ID_CA.C graphics routines.
///
/// The VGAGRAPH archive is indexed by VGAHEAD (chunk offsets) and
/// VGADICT (Huffman dictionary).  Each chunk is Huffman-compressed.
///
/// Relevant original constants live in GFXV_WL6.H (chunk enum).
use std::path::Path;

use anyhow::{bail, Result};


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

/// A decoded graphics chunk.  The exact format depends on the chunk type
/// (pic, sprite, font, etc.) — see GFXV_WL6.H for the enum layout.
pub struct GfxChunk {
    pub index: usize,
    pub data: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

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

pub struct GraphicsCache {
    /// Raw decoded chunks, indexed by chunk number.
    chunks: Vec<Option<Vec<u8>>>,
    /// Sprite metadata table (loaded from the sprite info chunk).
    pub sprites: Vec<SpriteInfo>,
}

impl GraphicsCache {
    pub fn load(base: &Path) -> Result<Self> {
        let base_str = base.to_str().unwrap();
        let header_path = format!("{}/VGAHEAD.WL6", base_str);
        let huffman_dict_path = format!("{}/VGADICT.WL6", base_str);
        let assets_path = format!("{}/VGAGRAPH.WL6", base_str);

        // TODO: locate VGAHEAD.WL6, VGADICT.WL6, VGAGRAPH.WL6 in `base`
        // and decode the Huffman-compressed archive.
        //
        // Steps:
        //   1. Read VGAHEAD to get chunk count and offsets.
        //   2. Read VGADICT for the 256-entry Huffman decode tree.
        //   3. For each chunk: read compressed bytes, decode with tree.
        //   4. Parse sprite table from chunk STARTSPRITES (see GFXV_WL6.H).


        let data_offsets: Vec<u32>;
        let huffman_tree: Vec<HuffmanNode>;
        let palette: Vec<(u8, u8, u8)> = load_game_palette("assets/GAMEPAL.OBJ");

        data_offsets = read_vga_file_offsets(&header_path).expect("failed to read data offsets");
        huffman_tree = read_vga_huffman_tree(&huffman_dict_path).expect("failed to load huffman tree");

        let raw_graphics_data: Vec<u8> = fs::read(assets_path).expect("failed to read file {path}"); 
        let mut decompressed: Vec<Vec<u8>> = Vec::new();

        for (i, pair) in data_offsets.windows(2).enumerate() {
            let start = pair[0] as usize;
            let end = pair[1] as usize;
            let chunk: &[u8] = &raw_graphics_data[start..end];
            println!("decompressing chunk {i}, size {}", chunk.len());
            let d = decompress_graphics_chunk(chunk, &huffman_tree);
            println!("decompressed chunk {i}, size {}", d.len());

            decompressed.push(d);
        }

        let mut chunks = Vec::<Option<Vec<u8>>>::new();

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
            
            chunks.push(Some(pic.clone()));
        }

        // TODO: build sprites vector from 


        log::warn!("GraphicsCache::load — stub, no data loaded from {:?}", base);
        Ok(Self { chunks, sprites: Vec::new() })
    }

    /// Return the raw bytes of chunk `index`, if loaded.
    pub fn chunk(&self, index: usize) -> Option<&[u8]> {
        self.chunks.get(index)?.as_deref()
    }

    /// Return a 32-bit RGBA pixel slice for a picture chunk.
    /// Width and height come from the picture table.
    pub fn pic_rgba(&self, _index: usize) -> Option<(&[u8], u16, u16)> {
        // TODO: convert chunky VGA palette data → RGBA8888
        None
    }
}




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

/*
reference code from C
//	File specific variables
	char			PageFileName[13] = {"VSWAP."};
	int				PageFile = -1;
	word			ChunksInFile;
	word			PMSpriteStart,PMSoundStart;


typedef	struct
		{
			longword	offset;		// Offset of chunk into file
			word		length;		// Length of the chunk

			int			xmsPage;	// If in XMS, (xmsPage * PMPageSize) gives offset into XMS handle

			PMLockType	locked;		// If set, this page can't be purged
			int			emsPage;	// If in EMS, logical page/offset into page
			int			mainPage;	// If in Main, index into handle array

			longword	lastHit;	// Last frame number of hit
		} PageListStruct;

//	General usage variables
	boolean			PMStarted,
					PMPanicMode,
					PMThrashing;
	word			XMSPagesUsed,
					EMSPagesUsed,
					MainPagesUsed,
					PMNumBlocks;
	long			PMFrameCount;
	PageListStruct	far *PMPages,
					_seg *PMSegPages;
PML_OpenPageFile(void)
{
	int				i;
	long			size;
	void			_seg *buf;
	longword		far *offsetptr;
	word			far *lengthptr;
	PageListStruct	far *page;

	PageFile = open(PageFileName,O_RDONLY + O_BINARY);
	if (PageFile == -1)
		Quit("PML_OpenPageFile: Unable to open page file");

	// Read in header variables
	read(PageFile,&ChunksInFile,sizeof(ChunksInFile));
	read(PageFile,&PMSpriteStart,sizeof(PMSpriteStart));
	read(PageFile,&PMSoundStart,sizeof(PMSoundStart));

	// Allocate and clear the page list
	PMNumBlocks = ChunksInFile;
    // allocate a chunk of memory to handle `PMNumBlocks` number of 
    // PageListStruct instances
	MM_GetPtr(&(memptr)PMSegPages,sizeof(PageListStruct) * PMNumBlocks);
    // prevent that memory from being allocated to anything else?
	MM_SetLock(&(memptr)PMSegPages,true);
    // PMPages is basically a C array, but beccause this is C
    // it's a pointer, and the next line essentially casts our
    // allocated chunk of memory as an array of PageListStructs
	PMPages = (PageListStruct far *)PMSegPages;
    // Now we fill that whole allocated chunk of memory with 0s
	_fmemset(PMPages,0,sizeof(PageListStruct) * PMNumBlocks);

	// Read in the chunk offsets

    // each chunk offset will be a `longword`, which in rust will be
    // a `u32`
	size = sizeof(longword) * ChunksInFile;
    // allocate `size` bytes into a void pointer called `buf``
	MM_GetPtr(&buf,size);
    // read `size` bytes from our file into the memory `buf`
    // points to.
    // basically `buf` points to an array of offsets, where each
    // `offset` is 32 bits and has `ChunksInFile` elements
	if (!CA_FarRead(PageFile,(byte far *)buf,size))
		Quit("PML_OpenPageFile: Offset read failed");
    // again, in C-ness, offsetptr will basically be an `array`
    // of `u32` with `ChunksInFile` elements.
	offsetptr = (longword far *)buf;
    // each iteration will add 1 to `i` and will increment
    // `page` by the number of bytes represented in a `PageListStruct`
	for (i = 0,page = PMPages;i < ChunksInFile;i++,page++)
        // assign the offset of each `page` to the current `u32`
        // in `buf` and then increment `buf` index (`offsetptr`) 
        // to the next entry in `buf`
		page->offset = *offsetptr++;
    // release `buf`
	MM_FreePtr(&buf);

	// Read in the chunk lengths
	size = sizeof(word) * ChunksInFile;
	MM_GetPtr(&buf,size);
	if (!CA_FarRead(PageFile,(byte far *)buf,size))
		Quit("PML_OpenPageFile: Length read failed");
	lengthptr = (word far *)buf;
	for (i = 0,page = PMPages;i < ChunksInFile;i++,page++)
		page->length = *lengthptr++;
	MM_FreePtr(&buf);
} */

enum PMLockType	{
    PMLUnlocked,    // in the original c, probably 0 at the byte level, but need to verify.
    PMLLocked,      // probably 1
} 
struct PageListStruct {
    offset: u32,
    length: u16,
    xms_page: i16,
    locked: PMLockType,
    ems_page: i16,
    main_page: i16,
    last_hit: u32,
}

fn read_page_file(path: &str) {
    let bytes = fs::read(path).expect("failed to read page file");
    let mut cursor = Cursor::new(bytes);

    let chunks_in_file: u16 = cursor.read_u16::<LE>().expect("failed to read chunks_in_file");
    let pm_sprite_start: u16 = cursor.read_u16::<LE>().expect("failed to read pm_sprite_start");
    let pm_sound_start: u16 = cursor.read_u16::<LE>().expect("failed to read pm_sound_start");

    let pm_num_blocks = chunks_in_file;

    // this will end up with `chunks_in_file` elements
    let mut offsets = Vec::<u32>::new();
    let mut pm_pages = Vec::<PageListStruct>::new();

    // the next chunk of memory is a series of 4-byte chunks
    // with `chunks_in_file` elements, where each 4-byte chunk
    // is the offset of a chunk of data that will become a 
    // PageListStruct instance.
    for _ in 0..chunks_in_file {
        let offset = cursor.read_u32::<LE>().expect("failed to read offset");
        pm_pages.push(PageListStruct {
            offset,
            length: 0,
            xms_page: 0,
            locked: PMLockType::PMLUnlocked,
            ems_page: 0,
            main_page: 0,
            last_hit: 0,
        })
    }

    // the next chunk of memory is a series of 2-byte chunks,
    // also with `chunks_in_file` elements, where each 2-byte chunk
    // is the length of the chunk of data that becomes a
    // PageListStruct instance.
    // together with `offset` above, `length` helps us identify the
    // exact location and size of each chunk.
    for i in 0..(chunks_in_file as usize) {
        let length = cursor.read_u16::<LE>().expect("failed to read length");
        pm_pages[i].length = length;
    }


}