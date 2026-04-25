struct FontStruct {
    height: i16,
    location: [i16; 256]
    width: [u8; 256],
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

const LINE_WIDTH: usize = 80; // 320px / 4 planes


impl TextDrawContext {
    pub fn measure_string(&self, s: &str, font: &FontStruct) -> (i16, i16) {
        let height = font.height;
        let mut width: i16 = 0;

        for b in s.bytes() {
            // in case we get \0 terminated strings, since this
            // is a port of an old C program
            if b == 0 {
                break;
            }
            width += font.width[b as usize] as i16;
        }

        (width, height)
    }

    pub fn draw_string(&mut self, s: &str, fb: &[u8], font: &FontStruct) -> TextDrawResult {
        // this is just a stub until we have the actual font data
        let font_data: Vec<u8> = Vec::new();

        // int		width,step,height,i;
        // byte	far *source, far *dest, far *origdest;
        // byte	far *col_src, far *col_dest;
        let col_src: &u8;
        let col_dest: &u8;
        let mask: u8;

        let height = font.height;
        let buffer_height = font.height;

        // formerly `dest` in the c code
        let dest = self.py as usize * LINE_WIDTH + (self.px as usize >> 2);
        mask = 1 << (self.px & 3);

        for b in s.bytes() {
            // break on \0 string terminator
            if b == 0 {
                break;
            }
            let width = font.width[b as usize] as i16;
            let step = width;
            let offset = font.location[b as usize] as i16;
            let source = &font_data[offset];

            while width > 0 {
                width -= 1;
                fb[dest + (self.px & 3) as usize] = self.font_color;
                col_src  = source;
                col_dest = dest;
                for row in 0..height {
                    if *col_src > 0 {
                        fb[dest_offset] = self.font_color;
                    }
                // 	if (*col_src)
                // 		*col_dest = fontcolor;
                // 	col_src  += step;
                // 	col_dest += linewidth;
                }
                // source++;
                // px++;
                // mask <<= 1;
                // if (mask == 16)
                // {
                // 	mask = 1;
                // 	dest++;
                // }
            }
        }
    }
}
