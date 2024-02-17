// Screen UI Manager

pub struct Manager {
    lcd: super::lcd::LcdDisplay,

    spectrum_y: u16,
}

impl Manager {
    pub fn new(lcd: super::lcd::LcdDisplay) -> Self {
        Self {
            lcd,
            spectrum_y: 0,
        }
    }

    pub fn init(&mut self) {
        self.lcd.init();
    }

    pub fn draw_text(&mut self, text: &[u8], x: u16, y: u16) {
        let renderer = super::text::TextRenderer::new(text);
        let size = renderer.size();
        self.lcd.set_window(x, y, size.0, size.1);
        self.lcd.send_data_iter(renderer.render());
    }

    pub fn draw_spectrum(&mut self, data: &[crate::dsp::DSPComplex]) {
        self.lcd.set_window(10, self.spectrum_y + 64, data.len() as u16, 1+1);
        defmt::info!("spectrum draw y = {}, len = {}", self.spectrum_y, data.len());
        for d in data {
            // let v = (d.norm().0.abs() >> 8) as u8;
            // let v = (d.norm().0.unsigned_abs() as u8 >> 3) as u16;
            let re = (d.re.0 >> 3).unsigned_abs().min(31) << 1;
            let im = (d.im.0 >> 3).unsigned_abs().min(31);
            // let v = re * re + im * im;
            let v = 31 | (re << 6) | (im << 11);
            self.lcd.send_data(&v.to_be_bytes());
        }
        self.spectrum_y += 1;
        if self.spectrum_y >= 128 {
            self.spectrum_y = 0;
        }

    }
}
