#![no_std]

pub const SAMPLE_RATE: usize = 192_000;

use defmt_rtt as _;
use panic_probe as _;

pub use rp2040_hal as hal;
pub use rp_pico as bsp;

pub mod board;

pub mod clockctl;
pub mod codec;
pub mod control;
pub mod core;
pub mod display;
pub mod dsp;
pub mod i2c;
pub mod sdr;
pub mod util;
