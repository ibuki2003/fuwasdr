// TLV320AIC3204
#![allow(dead_code)]
use crate::{
    dsp::DSPComplex,
    hal,
    i2c::SHARED_I2CBUS,
};
use hal::pio::PIOExt;
use hal::{pac, pio::PIOBuilder};

use crate::board::*;
use embedded_hal::i2c::I2c;

type PIODevice = pac::PIO0;
type SmClk = (PIODevice, hal::pio::SM0);
type SmI2s = (PIODevice, hal::pio::SM1);
pub type Rx = hal::pio::Rx<SmI2s>;


pub struct Codec {
    pin_mclk: PinCodecMclk,
    pin_bclk: PinCodecBclk,
    pin_wclk: PinCodecWclk,

    pin_din: PinCodecMfp1,
    pin_dout: PinCodecMfp2,
    _pin_mfp3: PinCodecMfp3,
    _pin_mfp4: PinCodecMfp4,
    _pin_mfp5: PinCodecMfp5,

    sm_clk: hal::pio::StateMachine<SmClk, hal::pio::Running>,
    sm_i2s: hal::pio::StateMachine<SmI2s, hal::pio::Running>,
    sm_i2s_rx: Option<Rx>,
    sm_i2s_tx: hal::pio::Tx<SmI2s>,
}

impl Codec {
    pub const I2C_ADDR: u8 = 0b001_1000;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pin_mclk: PinCodecMclk,
        pin_bclk: PinCodecBclk,
        pin_wclk: PinCodecWclk,
        pin_mfp1: PinCodecMfp1,
        pin_mfp2: PinCodecMfp2,
        pin_mfp3: PinCodecMfp3,
        pin_mfp4: PinCodecMfp4,
        pin_mfp5: PinCodecMfp5,
        pio: PIODevice,
        resets: &mut pac::RESETS,
    ) -> Self {
        let pin_din = pin_mfp1;
        let pin_dout = pin_mfp2;

        let (mut pio_, sm0, sm1, _, _) = pio.split(resets);

        let program_clk = pio_
            .install(
                &pio_proc::pio_asm![".wrap_target", "set pins, 1", "set pins, 0", ".wrap"].program,
            )
            .unwrap();
        let (mut sm_clk, _, _) = PIOBuilder::from_program(program_clk)
            .set_pins(pin_mclk.id().num, 1)
            .clock_divisor_fixed_point(800 >> 8, (800 & 0xff) as u8)
            .build(sm0);
        sm_clk.set_pindirs([(pin_mclk.id().num, hal::pio::PinDir::Output)]);

        // transceive LJF format
        let program_i2s = pio_
            .install(
                &pio_proc::pio_asm![
                    ".wrap_target",
                    // prepare data
                    "  pull noblock",
                    // left ch
                    "  wait 1 gpio 4", // wait for WCLK 1(Left)
                    "  set x, 15",
                    "left:",
                    "  wait 0 gpio 3", // wait for BCLK 0
                    "  out pins, 1",
                    "  in pins, 1",
                    "  wait 1 gpio 3", // wait for BCLK 1
                    "  jmp x-- left",
                    "  set pins, 0", // set data to 0
                    // right ch
                    "  wait 0 gpio 4", // wait for WCLK 0(Right)
                    "  set x, 15",
                    "right:",
                    "  wait 0 gpio 3", // wait for BCLK 0
                    "  out pins, 1",
                    "  in pins, 1",
                    "  wait 1 gpio 3", // wait for BCLK 1
                    "  jmp x-- right",
                    "  set pins, 0", // set data to 0
                    // push data
                    "  push noblock",
                    ".wrap",
                ]
                .program,
            )
            .unwrap();
        let (mut sm_i2s, rx, tx) = PIOBuilder::from_program(program_i2s)
            .out_pins(pin_din.id().num, 1)
            .in_pin_base(pin_dout.id().num)
            .set_pins(pin_din.id().num, 1)
            .buffers(hal::pio::Buffers::RxTx)
            .out_shift_direction(hal::pio::ShiftDirection::Left)
            .in_shift_direction(hal::pio::ShiftDirection::Left)
            // .clock_divisor_fixed_point(1, 0) // should work fast enough (at least BCLK*4)
            .build(sm1);
        sm_i2s.set_pindirs([
            (pin_bclk.id().num, hal::pio::PinDir::Input),
            (pin_wclk.id().num, hal::pio::PinDir::Input),
            (pin_din.id().num, hal::pio::PinDir::Output),
            (pin_dout.id().num, hal::pio::PinDir::Input),
        ]);

        Self {
            pin_mclk,
            pin_bclk,
            pin_wclk,

            pin_din,
            pin_dout,
            _pin_mfp3: pin_mfp3,
            _pin_mfp4: pin_mfp4,
            _pin_mfp5: pin_mfp5,

            sm_clk: sm_clk.start(),
            sm_i2s: sm_i2s.start(),
            sm_i2s_rx: Some(rx),
            sm_i2s_tx: tx,
        }
    }

    pub fn init(&mut self) {
        // init chip
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            let chunks = &[
                // Reset
                &[0x00, 0x00], // page 0
                &[0x01, 0x01], // soft reset
                // PLL
                &[0x04, 0x03], // Low Range, from MCLK, to CODEC_CLKIN
                &[0x05, (0b1 << 7 | 1 << 4 | 1 << 0)], // power up, P = 1, R = 1
                &[0x06, 4],    // J = 4
                &[0x07, (9152 >> 8) as u8], // D = 9152
                &[0x08, (9152 & 0xff) as u8],
                // DAC Clock
                &[0x0b, 1 << 7 | 2], // NDAC = 2
                &[0x0c, 1 << 7 | 8], // MDAC = 8
                &[0x0d, 0],          // DOSR = 32
                &[0x0e, 32],
                // &[0x19, 0x00], // PLL_CLKIN = MCLK // CDIV?
                &[0x1b, 0b11001100], // Interface LJF, 16bit, BCLK out, WCLK out, DOUT no Hi-Z
                // &[0x1c, 0],          // offset 0
                &[0x1d, 0b00000100], // BDIV_CLKIN = DAC_CLK (fs * 32 * 8)
                &[0x1e, 1 << 7 | 2], // BCLK DIV 2
                &[0x3c, 17],         // DAC proc block = 17
                // setup ADC
                &[0x12, 2],          // NADC disable; ADC_CLK := DAC_CLK
                &[0x13, 1 << 7 | 4], // MADC = 4 from ADC_CLK
                &[0x14, 64],         // AOSR = 64
                &[0x21, 0x00],       // dout
                &[0x3d, 0x01],       // ADC PRB_R1 (Filter A, 1 IIR, AGC)
                &[0x35, 0b00010010], // Bus Keeper dis, DOUT is Primary DOUT
                &[0x36, 0b01 << 1],  // DIN is primary din
                &[0x53, 40],         // Left 20dB
                &[0x54, 40],         // Right 20dB
                &[0x56, 0x00],       // AGC Disabled
                &[0x5e, 0x00],       // AGC Disabled
                // routing
                &[0x00, 0x01], // page 1
                &[0x01, 0x08], // enable AVdd LDO
                &[0x02, 0x01], // enable master analog power control
                // DAC analog blocks
                &[0x14, 0x25], // HP startup time
                &[0x0c, 0x08], // DAC to HP
                &[0x0d, 0x08],
                &[0x03, 0x00], // DAC PTM_P3/4
                &[0x04, 0x00],
                &[0x10, 0x0a], // DAC gain 10dB
                &[0x11, 0x0a],
                &[0x09, 0x30], // Power up HPL/HPR
                // ADC analog
                &[0x0a, 0x00], // input common mode 0.9V
                &[0x3d, 0x00], // select ADC PTM_R4
                &[0x47, 0x32], // MicPGA startup delay 3.1ms
                &[0x7b, 0x01], // REF charging 40ms
                &[0x34, 0x10], // Route IN2L to LEFT_P with 10K
                &[0x36, 0x10], // Route IN2R to LEFT_N with 10k
                &[0x37, 0x40], // Route IN1R to RIGHT_P with 10k
                &[0x39, 0x10], // Route IN1L to RIGHT_N with 10k
                &[0x3b, 72],   // L MICPGA: unmute, set gain to (72/2)dB
                &[0x3c, 72],   // R MICPGA: same as L
            ];

            for chunk in chunks {
                i2c.write(Self::I2C_ADDR, *chunk).unwrap();
            }

            cortex_m::asm::delay(125_000 * 10); // about 10ms

            let chunks = &[
                &[0x00, 0x00],       // page 0
                &[0x3f, 0b11010100], // powerup LR DAC
                &[0x40, 0b00000000], // unmute dac digial volume
                &[0x51, 0b11000000], // powerup LR ADC
                &[0x52, 0x00],       // unmute adc digial volume
            ];
            for chunk in chunks {
                i2c.write(Self::I2C_ADDR, *chunk).unwrap();
            }
        });

        // setup DMA
    }

    pub fn take_rx(&mut self) -> Option<Rx> {
        self.sm_i2s_rx.take()
    }

    pub fn try_write_sample(&mut self, v: DSPComplex) -> bool {
        self.sm_i2s_tx
            .write(unsafe { *(&v as *const DSPComplex as *const u32) })
    }

    pub fn set_agc_target(&mut self, v: u8) {
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            let vv = 0b10000011 | ((v & 0b111) << 4);
            i2c.write(Self::I2C_ADDR, &[0x00, 0x00]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x56, vv]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x5e, vv]).unwrap();
        });
    }

    pub fn set_adc_gain(&mut self, v: u8) {
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            i2c.write(Self::I2C_ADDR, &[0x00, 0x01]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x3b, v]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x3c, v]).unwrap();
        });
    }

    pub fn set_volume(&mut self, v: i8) {
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            let vv = (v as u8) >> 1;
            i2c.write(Self::I2C_ADDR, &[0x00, 0x00]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x53, vv]).unwrap();
            i2c.write(Self::I2C_ADDR, &[0x54, vv]).unwrap();
        });
    }

    pub fn get_agc_gain(&mut self) -> (u8, u8) {
        critical_section::with(|cs| {
            let mut rc = SHARED_I2CBUS.borrow(cs).borrow_mut();
            let i2c = rc.as_mut().unwrap();
            let mut buf = [0; 2];
            i2c.write(Self::I2C_ADDR, &[0x00, 0x00]).unwrap();
            i2c.write_read(Self::I2C_ADDR, &[0x5d], &mut buf[0..1])
                .unwrap();
            i2c.write_read(Self::I2C_ADDR, &[0x65], &mut buf[1..2])
                .unwrap();
            (buf[0], buf[1])
        })
    }
}

/*
Fs = 192kHz
AOSR = 64!

NDAC = 2
MDAC = 4
MCLK = 125MHz / 2 / (800/256) = 20MHz
J.D = 4.9152
P = R = 1

x = [(i, 10000*98304*i%(125000*128)) for i in range(256, 2560)]
print('\n'.join(map(str, ((math.gcd(2**30,i[0])), i[0]) for i in x if i[1] == 0))))

*/
