use std::{fs, path::Path};
use serde::Deserialize;
use std::collections::HashMap;

use anyhow::{Context, Error};
use image::GenericImageView;

use crate::{SCREEN_WIDTH, assets};

#[derive(Deserialize)]
struct FontGlyph {
    x: u16,
    width: u16,
}

#[derive(Deserialize)]
struct FontMetadata {
    height: u16,
    altas_width: u16,
    glyphs: HashMap<u16, FontGlyph>,
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

    fn character(&self, ascii: u16) -> (u16, u16) {
        (self.metadata.glyphs[&ascii].x, self.metadata.glyphs[&ascii].width)
    }
}

pub struct TextDrawContext {
    pub font_color: [u8; 4],
    pub back_color: [u8; 4],
    pub current_font: i16,
    pub fonts: [FontStruct; 2],
}

impl TextDrawContext {
    pub fn new(base_path: &Path) -> Result<Self, Error> {
        // load font 1 and font 2
        let font1 = FontStruct::new_from_file(base_path, "font_1")?;
        let font2 = FontStruct::new_from_file(base_path, "font_2")?;

        Ok(Self {
            font_color: assets::colors::TEXTCOLOR,
            back_color: assets::colors::BKGDCOLOR,
            current_font: 0,
            fonts: [font1, font2],
        })  
    }

    fn font_from_number(&self, font_number: u16) -> Result<&FontStruct, Error> {
        match font_number {
            1 => Ok(&self.fonts[0]),
            2 => Ok(&self.fonts[1]),
            _ => anyhow::bail!("invalid font number {font_number}"),
        }
    }

    /// renders a string of characters in a single row
    pub fn draw_string(&mut self, s: &str, fb: &mut [u8], dest_x: u16, dest_y: u16, font_number: u16) -> Result<(u16, u16), Error> {
        let fb_width = SCREEN_WIDTH as u16;
        let character_spacing: u16 = 3;

        let mut fb_offset_x = dest_x;

        let font = self.font_from_number(font_number)?;

        let src_bytes = font.data.as_raw();

        for b in s.bytes() {
            // break on \0 string terminator
            if b == 0 {
                break;
            }
            let (offset, width) = font.character(b as u16);
            
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

        Ok((fb_offset_x, font.metadata.height))
    }
}
