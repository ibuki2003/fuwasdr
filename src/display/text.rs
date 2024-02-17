// text renderer

const EN_FONT_ORIGIN: *const u16 = (0x10000000 | 0x1fe800) as *const u16;
const EN_FONT_HEIGHT: u8 = 32;
const EN_FONT_WIDTH: u8 = 16;

pub struct TextRenderer<'a> {
    text: &'a [u8],
}

impl<'a> TextRenderer<'a> {
    pub fn new(text: &[u8]) -> TextRenderer {
        TextRenderer { text }
    }

    pub fn size(&self) -> (u16, u16) {
        (
            self.text.len() as u16 * EN_FONT_WIDTH as u16,
            EN_FONT_HEIGHT as u16,
        )
    }

    pub fn render(self) -> TextRendererIter<'a> {
        TextRendererIter {
            text: self.text,
            row: 0,
            col: 0,
            byte: 0,
        }
    }
}

pub struct TextRendererIter<'a> {
    text: &'a [u8],
    row: u8,
    col: usize,
    byte: u8,
}

impl Iterator for TextRendererIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.row > EN_FONT_HEIGHT {
            return None;
        }
        let font_data = unsafe { core::slice::from_raw_parts(EN_FONT_ORIGIN, 32 * 16 * 6) };

        let char = self.text[self.col];

        let data = if (0x20..=0x7f).contains(&char) {
            let glyph =
                font_data[(char - 0x20) as usize * EN_FONT_HEIGHT as usize + self.row as usize];
            let bit = (glyph >> (self.byte >> 1)) & 1;

            if bit != 0 {
                // if self.byte & 1 != 0 { 0xff } else { 0xff }
                0xff
            } else {
                0
            }
        } else {
            0
        };

        self.byte += 1;
        if self.byte >= EN_FONT_WIDTH * 2 {
            self.byte = 0;
            self.col += 1;
        }

        if self.col >= self.text.len() {
            self.col = 0;
            self.row += 1;
        }

        Some(data)
    }
}
