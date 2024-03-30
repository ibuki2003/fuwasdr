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

pub struct EnTextRenderer<'a, F: AsciiFont> {
    text: &'a [u8],
    row: u8,
    col: usize,
    byte: u8,

    _phantom: core::marker::PhantomData<F>,
}

pub type TextRendererEN<'a> = EnTextRenderer<'a, FontEN>;
pub type TextRendererMisakiEn<'a> = EnTextRenderer<'a, FontMisaki>;

impl<'a, F: AsciiFont> EnTextRenderer<'a, F> {
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

impl<F: AsciiFont> Iterator for EnTextRenderer<'_, F> {
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

pub struct FontEN;
impl FontEN {
    const ORIGIN: *const u16 = (0x10000000 | 0x1cc000) as *const u16;
}
impl AsciiFont for FontEN {
    type Octet = u16;
    const HEIGHT: u8 = 32;
    const WIDTH: u8 = 16;

    fn data() -> &'static [u16] {
        unsafe { core::slice::from_raw_parts(Self::ORIGIN, Self::HEIGHT as usize * 96) }
    }
}

const MISAKI_BASE: *const u8 = (0x10000000 | 0x1ce000) as *const u8;
const fn misaki_offset(c: u8) -> usize {
    match c {
        0x00..=0x01 => 0,
        0x03..=0x05 => c as usize - 0x03 + 1,
        0x20..=0x27 => c as usize - 0x20 + 3,
        0x30..=0x31 => c as usize - 0x30 + 10,
        0x32..=0x34 => c as usize - 0x32 + 11,
        0x4e..=0xa0 => c as usize - 0x4e + 13,
        0xff => 95,
        _ => unreachable!(),
    }
}
const fn misaki_idx(c: u16) -> usize {
    (misaki_offset((c >> 8) as u8) << 8 | (c & 0xff) as usize) * 8
}

// render full-width chars
pub struct FontMisaki;
impl FontMisaki {
    const ORIGIN: *const u8 = MISAKI_BASE.wrapping_add((misaki_offset(0xff) << 8) * 8);
}
impl AsciiFont for FontMisaki {
    type Octet = u8;
    const HEIGHT: u8 = 8;
    const WIDTH: u8 = 8;

    fn data() -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(Self::ORIGIN, Self::HEIGHT as usize * 96) }
    }
}

pub struct MisakiTextRender<'a> {
    text: &'a [u16],
    row: u8,
    col: usize,
    byte: u8,

    curr_idx: usize,
}

impl<'a> MisakiTextRender<'a> {
    pub fn new(text: &'a [u16]) -> Self {
        Self {
            text,
            row: 0,
            col: 0,
            byte: 0,

            curr_idx: misaki_idx(text[0]),
        }
    }

    pub fn size(&self) -> (u16, u16) {
        let w = self
            .text
            .iter()
            .fold(0, |acc, &c| acc + if c < 0x100 { 4 } else { 8 }) as u16;
        (w, 8)
    }

    #[inline]
    fn get_pixel(char_idx: usize, row: u8, col: u8) -> bool {
        let data = unsafe { MISAKI_BASE.add(char_idx) };
        ((unsafe { *data.add(row as usize) } >> col) & 1) != 0
    }
}

impl Iterator for MisakiTextRender<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= 8 {
            return None;
        }

        let pixel = Self::get_pixel(self.curr_idx, self.row, self.byte >> 1);

        let data = if pixel { 0xff } else { 0 };

        self.byte += 1;
        let width = if self.curr_idx < 0x100 * 8 { 4 } else { 8 };
        if self.byte >= width * 2 {
            self.byte = 0;
            self.col += 1;

            if self.col >= self.text.len() {
                self.col = 0;
                self.row += 1;
            }
            self.curr_idx = misaki_idx(self.text[self.col]);
        }

        Some(data)
    }
}
