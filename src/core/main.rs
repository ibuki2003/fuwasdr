use defmt::*;
use hal::{Sio, Watchdog};

use crate::hal;
use hal::{gpio::Pins, pac};

pub fn main() -> ! {
    info!("Hello, world!");

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let core = pac::CorePeripherals::take().unwrap();

    const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
    let clocks = hal::clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    loop {}
}
