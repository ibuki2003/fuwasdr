use crate::board::*;
use crate::hal;
use embedded_hal::{
    digital::{OutputPin, StatefulOutputPin},
    spi::SpiBus,
};
use hal::fugit::HertzU32;

const SPI_BAUD: HertzU32 = HertzU32::MHz(50);

type LcdSpi =
    hal::spi::Spi<hal::spi::Enabled, LcdSpiDevice, (PinLcdMosi, PinLcdMiso, PinLcdSck), 8>;
pub struct LcdDisplay {
    spi: LcdSpi,
    pin_dc: PinLcdDcRs,
    pin_reset: PinLcdReset,
    _pin_touchirq: PinLcdTouchIrq,
    _pin_touchcs: PinLcdTouchCs,
    pin_dispcs: PinLcdDispCs,
}

impl LcdDisplay {
    pub const LCD_WIDTH: u16 = 320;
    pub const LCD_HEIGHT: u16 = 240;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spidev: LcdSpiDevice,
        pin_reset: PinLcdReset,
        pin_touchirq: PinLcdTouchIrq,
        pin_miso: PinLcdMiso,
        pin_touchcs: PinLcdTouchCs,
        pin_sck: PinLcdSck,
        pin_mosi: PinLcdMosi,
        pin_dispcs: PinLcdDispCs,
        pin_dc: PinLcdDcRs,
        resets: &mut hal::pac::RESETS,
        peripheral_clock: HertzU32,
    ) -> Self {
        let spi = hal::spi::Spi::new(spidev, (pin_mosi, pin_miso, pin_sck)).init(
            resets,
            peripheral_clock,
            SPI_BAUD,
            embedded_hal::spi::MODE_0,
        );
        Self {
            spi,
            pin_dc,
            pin_reset,
            _pin_touchirq: pin_touchirq,
            _pin_touchcs: pin_touchcs,
            pin_dispcs,
        }
    }

    pub fn init(&mut self) {
        self.pin_dispcs.set_low().unwrap();

        // HARD reset
        self.pin_reset.set_low().unwrap();
        cortex_m::asm::delay(125000); // about 1ms
        self.pin_reset.set_high().unwrap();
        cortex_m::asm::delay(125000 * 120); // at least 120ms

        // memory access ctl: landscape
        self.send_command(&[0x36, 0x28]); // R/C exchange, BGR

        // color mode: 16bit
        self.send_command(&[0x3a, 0x55]);

        // entry mode set
        self.send_command(&[0xB7, 0x06]);
        self.send_command(&[0xb6, 0x0a, 0x82, 0x27, 0x00]);

        // entry mode
        self.send_command(&[0xB7, 0x06]);
        // display function control
        self.send_command(&[0xB6, 0x0A, 0x82, 0x27, 0x00]);

        // sleep out
        self.send_command(&[0x11]);
        cortex_m::asm::delay(125000 * 60); // at least 60ms

        // display on
        self.send_command(&[0x29]);
        // cortex_m::asm::delay(125000 * 60); // at least 60ms

        // fill screen with black
        self.set_window(0, 0, Self::LCD_WIDTH, Self::LCD_HEIGHT);
        self.send_data_iter(
            core::iter::repeat(0).take(Self::LCD_WIDTH as usize * Self::LCD_HEIGHT as usize * 2),
        );
    }

    fn send_command(&mut self, command: &[u8]) {
        self.send_register(&command[0..1]);
        if command.len() > 1 {
            self.send_data(&command[1..]);
        }
    }

    fn send_register(&mut self, cmd: &'_ [u8]) {
        if self.pin_dc.is_set_high().unwrap() {
            embedded_hal::spi::SpiBus::flush(&mut self.spi).unwrap();
            self.pin_dc.set_low().unwrap();
        }
        self.spi.write(cmd).unwrap();
    }

    pub fn send_data(&mut self, cmd: &'_ [u8]) {
        if self.pin_dc.is_set_low().unwrap() {
            embedded_hal::spi::SpiBus::flush(&mut self.spi).unwrap();
            self.pin_dc.set_high().unwrap();
        }
        self.spi.write(cmd).unwrap();
    }

    pub fn send_data_iter(&mut self, data: impl Iterator<Item = u8>) {
        if self.pin_dc.is_set_low().unwrap() {
            embedded_hal::spi::SpiBus::flush(&mut self.spi).unwrap();
            self.pin_dc.set_high().unwrap();
        }
        for b in data {
            self.spi.write(&[b]).unwrap();
        }
    }

    pub fn send_data_unchecked(&mut self, cmd: &'_ [u8]) {
        self.spi.write(cmd).unwrap();
    }

    pub fn set_window(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.send_command(&[
            0x2A,
            (x >> 8) as u8,
            x as u8,
            ((x + w - 1) >> 8) as u8,
            (x + w - 1) as u8,
        ]);
        self.send_command(&[
            0x2B,
            (y >> 8) as u8,
            y as u8,
            ((y + h - 1) >> 8) as u8,
            (y + h - 1) as u8,
        ]);
        self.send_command(&[0x2c]);
    }

    pub fn set_scroll_pos(&mut self, y: u16) {
        self.send_command(&[0x37, (y >> 8) as u8, y as u8]);
    }
}
