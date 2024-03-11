// text renderer

use num::traits::AsPrimitive;

pub trait AsciiFont {
    type Octet: AsPrimitive<u32>;
    // const ORIGIN: *const Octet;
    const HEIGHT: u8;
    const WIDTH: u8;

    fn data() -> &'static [Self::Octet];

    fn glyph(c: u8) -> Option<&'static [Self::Octet]> {
        if (0x20..=0x7f).contains(&c) {
            Some(&Self::data()[(c - 0x20) as usize * Self::HEIGHT as usize..])
        } else {
            None
        }
    }

    fn get_pixel(c: u8, row: u8, col: u8) -> bool {
        match Self::glyph(c) {
            Some(glyph) => ((glyph[row as usize].as_() >> col) & 1) != 0,
            None => false,
        }
    }
}

pub struct FontEN;
impl FontEN {
    const ORIGIN: *const u16 = (0x10000000 | 0x1fe800) as *const u16;
}
impl AsciiFont for FontEN {
    type Octet = u16;
    const HEIGHT: u8 = 32;
    const WIDTH: u8 = 16;

    fn data() -> &'static [u16] {
        unsafe { core::slice::from_raw_parts(Self::ORIGIN, Self::HEIGHT as usize * 96) }
    }
}

pub struct FontMisaki;
impl FontMisaki {
    const ORIGIN: *const u8 = (0x10000000 | 0x100000) as *const u8;
}
impl AsciiFont for FontMisaki {
    type Octet = u8;
    const HEIGHT: u8 = 8;
    const WIDTH: u8 = 8;

    fn data() -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(Self::ORIGIN, Self::HEIGHT as usize * 96) }
    }
}

pub struct TextRenderer<'a, F: AsciiFont> {
    text: &'a [u8],
    row: u8,
    col: usize,
    byte: u8,

    _phantom: core::marker::PhantomData<F>,
}

pub type TextRendererEN<'a> = TextRenderer<'a, FontEN>;
pub type TextRendererMisaki<'a> = TextRenderer<'a, FontMisaki>;

impl<'a, F: AsciiFont> TextRenderer<'a, F> {
    pub fn new(text: &'a [u8]) -> Self {
        Self {
            text,
            row: 0,
            col: 0,
            byte: 0,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.text.len() as u16 * F::WIDTH as u16, F::HEIGHT as u16)
    }
}

impl<F: AsciiFont> Iterator for TextRenderer<'_, F> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= F::HEIGHT {
            return None;
        }

        let char = self.text[self.col];
        let pixel = F::get_pixel(char, self.row, self.byte >> 1);

        let data = if pixel { 0xff } else { 0 };

        self.byte += 1;
        if self.byte >= F::WIDTH * 2 {
            self.byte = 0;
            self.col += 1;

            if self.col >= self.text.len() {
                self.col = 0;
                self.row += 1;
            }
        }

        Some(data)
    }
}
