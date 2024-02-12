// Clock Generator Si5351A manipulation
use crate::i2c::SHARED_I2CBUS;
use embedded_hal::i2c::I2c;
use rp2040_hal::fugit::HertzU32;

const XTAL_FREQ: HertzU32 = HertzU32::MHz(25_u32);

pub struct ClockCtl {
    current_freq: HertzU32,
    current_tune_factors: TuneFactors,
}

pub enum Error {
    I2cError,
    DeviceInInitialization,
    InvalidValue,
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::I2cError => "I2C error",
            Self::DeviceInInitialization => "Device in initialization",
            Self::InvalidValue => "Invalid value",
        })
    }
}

impl ClockCtl {
    pub const I2C_ADDR: u8 = 0b110_0000;
    pub const XTAL_FREQ: HertzU32 = XTAL_FREQ;

    pub fn new() -> Self {
        Self {
            current_freq: HertzU32::MHz(0),
            current_tune_factors: TUNE_FACTORS[0],
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();

            let mut buf = [0; 1];
            i2c.write_read(Self::I2C_ADDR, &[0x00], &mut buf)
                .map_err(|_| Error::I2cError)?;
            if buf[0] & (1 << 7) != 0 {
                return Err(Error::DeviceInInitialization);
            }

            let chunks: &[&[u8]] = &[
                &[3, 0xff],        // disable all outputs
                &[15, 0x00],       // PLLx_SRC = 0, DIV=1
                &[149, 0, 0],      // spread spectrum disable
                &[183, 0b10 << 6], // CL = 8pF
                &[187, 0],         // CL = 8pF
                &[3, 0x00],        // enable all outputs again
            ];

            for chunk in chunks {
                i2c.write(Self::I2C_ADDR, chunk)
                    .map_err(|_| Error::I2cError)?;
            }
            Ok(())
        })
    }

    pub const fn make_clk_control(
        powered: bool,
        division_integer: bool, // 1: integer, 0: fractional
        source_b: bool,         // 0: PLLA, 1: PLLB
        invert: bool,
    ) -> u8 {
        (!powered as u8) << 7
            | (division_integer as u8) << 6
            | (source_b as u8) << 5
            | (invert as u8) << 4
            | 0b1111
    }

    pub fn get_current_freq(&self) -> HertzU32 {
        self.current_freq
    }

    pub fn tune(&mut self, target: HertzU32) -> Result<(), Error> {
        self.current_freq = target;
        let a = self.current_tune_factors.get_synth_param(target);
        if a != 0 {
            // just set pll
            self.set_plla_mul(a, self.current_tune_factors.c)?;
        } else {
            // change div
            self.current_tune_factors =
                TUNE_FACTORS[find_div_idx(target).ok_or(Error::InvalidValue)?];
            self.set_div(self.current_tune_factors.div)?;
            let a = self.current_tune_factors.get_synth_param(target);
            self.set_plla_mul(a, self.current_tune_factors.c)?;
        }

        Ok(())
    }

    fn set_div(&mut self, div: u32) -> Result<(), Error> {
        defmt::info!("setting div {}", div);
        if div <= 127 {
            let p1 = div - 4;
            let r0 = if div == 4 { 0b11 << 2 } else { (p1 >> 9) as u8 };
            critical_section::with(|cs| {
                let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
                let i2c = rc.as_mut().unwrap();
                let chunks: &[&[u8]] = &[
                    &[3, 0xff],        // disable all outputs
                    &[16, 0xc0, 0xc0], // power down
                    &[
                        42,
                        // MS0 (a = div * 128 - 512, b = 0, c = 1)
                        0,
                        1,
                        r0,
                        (p1 >> 1) as u8,
                        (p1 << 7) as u8,
                        0,
                        0,
                        0,
                        // MS1 (same as MS0)
                        0,
                        1,
                        r0,
                        (p1 >> 1) as u8,
                        (p1 << 7) as u8,
                        0,
                        0,
                        0,
                    ],
                    &[165, 0, div as u8], // PHOFF
                    &[177, 0xa0],         // pll reset
                    &[16, 0x4f, 0x4f],    // CLK0 power up TODO: drive strength
                    &[3, 0b00],           // enable all outputs
                ];

                for chunk in chunks {
                    i2c.write(Self::I2C_ADDR, chunk)
                        .map_err(|_| Error::I2cError)?;
                }
                Ok(())
            })
        } else {
            // todo!("set phase with hack");
            defmt::info!("not implemented. ignoring");
            Ok(())
        }
    }

    fn set_plla_mul(&mut self, b: u32, c: u32) -> Result<(), Error> {
        // here we use HW divider
        let (b, c) = crate::util::gcd::reduce(b, c);
        let p1: u32 = b * 128 / c - 512;
        let p2: u32 = b * 128 % c;

        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            i2c.write(
                Self::I2C_ADDR,
                &[
                    26, // MSNA
                    (c >> 8) as u8,
                    c as u8,
                    (p1 >> 16) as u8,
                    (p1 >> 8) as u8,
                    p1 as u8,
                    (c >> 16 << 4) as u8 | (p2 >> 16) as u8,
                    (p2 >> 8) as u8,
                    p2 as u8,
                ],
            )
            .map_err(|_| Error::I2cError)?;
            Ok(())
        })
    }
}

// (DIV, TS, A, B)
#[rustfmt::skip]
const TUNE_FACTORS: [TuneFactors; 27] = [
    TuneFactors::new_div(   4, 10, 1),
    TuneFactors {
        div: 6,
        ts: 10,
        c: 833_333,
        _mult: (5_000_000_u32 >> 6).wrapping_mul(XTAL_FREQ_INV_M),
        _mult_tz: 6,
    }, // rounded, not exact
    TuneFactors::new_div(   8, 10, 1),
    TuneFactors::new_div(  10, 10, 1),
    TuneFactors::new_div(  12, 10, 3),
    TuneFactors::new_div(  16,  5, 1),
    TuneFactors::new_div(  20,  5, 4),
    TuneFactors::new_div(  25,  1, 1),
    TuneFactors::new_div(  32,  1, 1),
    TuneFactors::new_div(  40,  1, 1),
    TuneFactors::new_div(  50,  1, 1),
    TuneFactors::new_div(  64,  1, 1),
    TuneFactors::new_div(  80,  1, 1),
    TuneFactors::new_div( 100,  1, 1),
    TuneFactors::new_div( 120,  1, 3),
    TuneFactors::new_div( 160,  1, 1),
    TuneFactors::new_div( 200,  1, 1),
    TuneFactors::new_div( 250,  1, 1),
    TuneFactors::new_div( 320,  1, 1),
    TuneFactors::new_div( 400,  1, 1),
    TuneFactors::new_div( 500,  1, 1),
    TuneFactors::new_div( 640,  1, 2),
    TuneFactors::new_div( 800,  1, 1),
    TuneFactors::new_div(1000,  1, 1),
    TuneFactors::new_div(1250,  1, 1),
    TuneFactors::new_div(1600,  1, 1),
    TuneFactors::new_div(1800,  1, 9),
];

// find optimal DIV for the target frequency
#[rustfmt::skip]
fn find_div_idx(target: HertzU32) -> Option<usize> {
    if target > HertzU32::MHz(225) { return None; }

    if target < HertzU32::Hz(333334) {
        // TODO: work with R Divider
        panic!("Frequency too low");
    }

    if target > HertzU32::MHz( 150) { return Some(0); }
    if target > HertzU32::MHz( 106) { return Some(1); }
    if target > HertzU32::MHz(  82) { return Some(2); }
    if target > HertzU32::MHz(  67) { return Some(3); }
    if target > HertzU32::MHz(  53) { return Some(4); }
    if target > HertzU32::MHz(  41) { return Some(5); }
    if target > HertzU32::MHz(  33) { return Some(6); }
    if target > HertzU32::MHz(  26) { return Some(7); }
    if target > HertzU32::MHz(  21) { return Some(8); }
    if target > HertzU32::MHz(  16) { return Some(9); }
    if target > HertzU32::MHz(  13) { return Some(10); }
    if target > HertzU32::MHz(  10) { return Some(11); }
    if target > HertzU32::kHz(8216) { return Some(12); }
    if target > HertzU32::kHz(6708) { return Some(13); }
    if target > HertzU32::kHz(5303) { return Some(14); }
    if target > HertzU32::kHz(4108) { return Some(15); }
    if target > HertzU32::kHz(3286) { return Some(16); }
    if target > HertzU32::kHz(2598) { return Some(17); }
    if target > HertzU32::kHz(2054) { return Some(18); }
    if target > HertzU32::kHz(1643) { return Some(19); }
    if target > HertzU32::kHz(1299) { return Some(20); }
    if target > HertzU32::kHz(1027) { return Some(21); }
    if target > HertzU32::kHz( 821) { return Some(22); }
    if target > HertzU32::kHz( 657) { return Some(23); }
    if target > HertzU32::kHz( 520) { return Some(24); }
    if target > HertzU32::kHz( 433) { return Some(25); }
    Some(26)
}

// F_OUT = F_XTAL * (A + B / C) / div
#[derive(Clone, Copy)]
struct TuneFactors {
    div: u32,
    ts: u8,
    c: u32,
    // for optimal calculation; (A*C + B) === a_mult * F_OUT / F_XTAL
    _mult: u32,
    _mult_tz: u8,
}

const XTAL_FREQ_INV_M: u32 = 585698849; // modular inverse of 25MHz
const XTAL_FREQ_TRAILING_ZEROS: u8 = 6;

impl TuneFactors {
    const MIN_MULT: u32 = 24;
    const MAX_MULT: u32 = 36;

    const fn new_div(div: u32, ts: u8, step: u32) -> Self {
        assert!(div > 0);
        assert!(XTAL_FREQ.to_Hz() * step % (div * ts as u32) == 0);
        let c = XTAL_FREQ.to_Hz() * step / (div * ts as u32);
        let _mult = div * c;
        let _mult_tz = _mult.trailing_zeros() as u8;
        let _mult = (_mult >> _mult_tz).wrapping_mul(XTAL_FREQ_INV_M);
        Self {
            div,
            ts,
            c,
            _mult,
            _mult_tz,
        }
    }

    #[inline]
    fn get_synth_param(&self, target: HertzU32) -> u32 {
        if target.to_Hz() * self.div > XTAL_FREQ.to_Hz() * Self::MAX_MULT {
            return 0;
        } // out of range
        if target.to_Hz() * self.div < XTAL_FREQ.to_Hz() * Self::MIN_MULT {
            return 0;
        } // out of range

        // target is assumed (ensured) to be divisible by ts
        let tz: u8 = target.to_Hz().trailing_zeros() as u8;
        debug_assert!(self._mult_tz + tz >= XTAL_FREQ_TRAILING_ZEROS);
        let v = (target.to_Hz() >> tz).wrapping_mul(self._mult);
        v << (self._mult_tz + tz - XTAL_FREQ_TRAILING_ZEROS)
    }
}
