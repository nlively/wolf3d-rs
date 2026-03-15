/// Graphics chunk loader — corresponds to ID_CA.C graphics routines.
///
/// The VGAGRAPH archive is indexed by VGAHEAD (chunk offsets) and
/// VGADICT (Huffman dictionary).  Each chunk is Huffman-compressed.
///
/// Relevant original constants live in GFXV_WL6.H (chunk enum).
use std::path::Path;

use anyhow::{bail, Result};

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
        // TODO: locate VGAHEAD.WL6, VGADICT.WL6, VGAGRAPH.WL6 in `base`
        // and decode the Huffman-compressed archive.
        //
        // Steps:
        //   1. Read VGAHEAD to get chunk count and offsets.
        //   2. Read VGADICT for the 256-entry Huffman decode tree.
        //   3. For each chunk: read compressed bytes, decode with tree.
        //   4. Parse sprite table from chunk STARTSPRITES (see GFXV_WL6.H).
        log::warn!("GraphicsCache::load — stub, no data loaded from {:?}", base);
        Ok(Self { chunks: Vec::new(), sprites: Vec::new() })
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
