use std::{fs, path::Path};
use serde::Deserialize;
use std::collections::HashMap;

use anyhow::Context;
use image::GenericImageView;

use crate::SCREEN_WIDTH;

struct FontGlyph {
    x: u8,
    width: u8,
}

struct FontMetadata {
    height: u8,
    altas_width: u8,
    glyphs: HashMap<u8, FontGlyph>,
}

struct FontStruct {
    data: image::RgbaImage,
    metadata: FontMetadata,
}

impl FontStruct {
    fn new_from_file(base_path: &std::path::Path, font_name: &str) -> Result<Self, Error> {
        let metadata_path = base_path.join(format!("{}.json", font_name));
        let image_path = base_path.join(format!("{}.png", font_name));

        let metadata_json = fs::read_to_string(metadata_path)?;

        let img = image::open(image_path)?;
        let metadata: FontMetadata = serde_json::from_str(metadata_json.as_str())?;

        // let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        Ok(Self {
            data: rgba,
            metadata,
        })
    }

    fn character(&self, ascii: u8) -> (u8, u8) {
        (self.metadata.glyphs[ascii].x, self.metadata.glyphs[ascii].width)
    }
}

struct TextDrawContext {
    pub px: i16,
    pub py: i16,
    pub font_color: u8,
    pub back_color: u8,
    pub font_number: i16,
}

struct TextDrawResult {
    pub buffer_width: i16,
    pub buffer_height: i16,
}

impl TextDrawContext {
    /// renders a string of characters in a single row
    pub fn draw_string(&mut self, s: &str, fb: &mut [u8], dest_x: u16, dest_y: u16, font: &FontStruct) -> Result<(), Error> {
        let fb_width = SCREEN_WIDTH;
        let character_spacing: u8 = 3;

        let mut fb_offset_x = dest_x;

        let height = font.height;

        let src_bytes = font.data.as_raw();

        for b in s.bytes() {
            // break on \0 string terminator
            if b == 0 {
                break;
            }
            let (offset, width) = font.character(b);
            
            for row in 0..font.metadata.height {
                let sy = row; // source y has no offset because the font image is just 1 row of glyphs
                let dy = dest_y + row;

                let src_start = ((sy * font.metadata.altas_width + offset) * 4) as usize;
                let src_end = src_start + (width * 4) as usize;

                let dst_start = ((dy * fb_width + fb_offset_x) * 4) as usize;
                let dst_end = dst_start + (width * 4) as usize;

                fb[dst_start..dst_end].copy_from_slice(&src_bytes[src_start..src_end]);
            }

            fb_offset_x += width + character_spacing;
        }

        Ok(())
    }
}
