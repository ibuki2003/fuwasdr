// Screen UI Manager

use crate::display::LcdDisplay;

pub struct Manager {
    lcd: super::lcd::LcdDisplay,

    spectrum_y: u16,
}

impl Manager {
    const FREQ_X: u16 = 64;
    const FREQ_Y: u16 = 0;
    const WF_X: u16 = 32;
    const WF_Y: u16 = 64;

    pub fn new(lcd: LcdDisplay) -> Self {
        Self { lcd, spectrum_y: 0 }
    }

    pub fn init(&mut self) {
        self.lcd.init();
    }

    pub fn draw_text(&mut self, text: &[u8], x: u16, y: u16) {
        let renderer = super::text::TextRendererEN::new(text);
        let size = renderer.size();
        self.lcd.set_window(x, y, size.0, size.1);
        self.lcd.send_data_iter(renderer);
    }

    pub fn draw_freq(&mut self, freq: u32) {
        let mut buf = [0u8; 9];

        let mut f0 = freq;
        for j in (0..9).rev() {
            buf[j] = (f0 % 10) as u8 + b'0';
            f0 /= 10;
            if f0 == 0 {
                break;
            }
        }

        for i in 0..3 {
            self.draw_text(
                &buf[i * 3..i * 3 + 3],
                Self::FREQ_X + (16 * 3 + 8) * i as u16,
                Self::FREQ_Y,
            );
        }

        // waterfall
        // interval = 100kHz
        // in screen = 133px

        self.lcd
            .set_window(0, Self::WF_Y - 16, LcdDisplay::LCD_WIDTH, 16);

        self.lcd
            .send_data_iter(core::iter::repeat(0x00).take(LcdDisplay::LCD_WIDTH as usize * 8 * 2));
        let x = Self::WF_X + 128 - (freq % 10_000 * 256 / 192000) as u16 - 120;

        for _ in 0..8 {
            for _ in 0..Self::WF_X {
                self.lcd.send_data_unchecked(&[0, 0]);
            }
            for i in Self::WF_X..Self::WF_X + 256 {
                if i >= x && (i - x) * 3 % 40 < 3 {
                    self.lcd.send_data_unchecked(&[0xff, 0xff]);
                } else {
                    self.lcd.send_data_unchecked(&[0, 0]);
                }
            }
            for _ in 0..Self::WF_X {
                self.lcd.send_data_unchecked(&[0, 0]);
            }
        }


        buf[4] = b'.';
        buf[6] = b'M';
        let mut f = (freq - 96_000 + 100_000 - 1) / 100_000;
        let mut x =
            (Self::WF_X as i32 + 128 + ((f * 100_000) as i32 - freq as i32) * 256 / 192000) as u16;
        while f * 100_000 < freq + 96_000 {
            let mut f0 = f;

            buf[5] = (f0 % 10) as u8 + b'0';
            f0 /= 10;

            let mut i = 0;
            for j in (0..4).rev() {
                buf[j] = (f0 % 10) as u8 + b'0';
                f0 /= 10;
                if f0 == 0 {
                    i = j;
                    break;
                }
            }

            let text = super::text::TextRendererMisaki::new(&buf[i..7]);

            self.lcd
                .set_window(x + 1 - (4 - i) as u16 * 8, Self::WF_Y - 16, text.size().0, text.size().1);
            self.lcd.send_data_iter(text);

            f += 1;
            x += 133;
        }
    }

    pub fn draw_cursor(&mut self, ts_k: u8) {
        self.lcd
            .set_window(Self::FREQ_X, Self::FREQ_Y + 32, 16 * 9 + 8 * 2, 1);
        for i in (0..3).rev() {
            for j in (0..3).rev() {
                self.lcd.send_data_iter(
                    core::iter::repeat(if i * 3 + j == ts_k { 0xff } else { 0x00 }).take(16 * 2),
                );
            }
            self.lcd
                .send_data_iter(core::iter::repeat(0x00).take(8 * 2));
        }
    }

    pub fn draw_spectrum(&mut self, data: &[crate::dsp::DSPComplex]) {
        self.lcd.set_window(
            Self::WF_X,
            self.spectrum_y + Self::WF_Y,
            data.len() as u16,
            1,
        );
        for d in data {
            let re = (d.re.0 >> 3).min(255).unsigned_abs();
            let im = (d.im.0 >> 3).min(255).unsigned_abs();
            let v = re * re + im * im;
            // let v = 31 | (re.min(31) << 6) | (im.min(31) << 11);
            let v = colormap(v);
            self.lcd.send_data(&v.to_be_bytes());
        }
        self.spectrum_y += 1;
        if self.spectrum_y >= LcdDisplay::LCD_HEIGHT - Self::WF_Y {
            self.spectrum_y = 0;
        }
    }
}

const COLOR_SHIFT: u16 = 4;
// map from 0..=65535
fn colormap(v: u16) -> u16 {
    if v >= (1 << (16 - COLOR_SHIFT)) {
        return 0xffff;
    }
    let v = v << COLOR_SHIFT;
    match v {
        0..=16383 => v >> 9,
        16384..=49151 => ((v - 16384) >> 4) | 31,
        49152..=65535 => ((v - 49152) << 2) | 2047,
    }
}
