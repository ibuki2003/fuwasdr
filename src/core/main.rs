use defmt::*;
use hal::{Sio, Watchdog};

use crate::{hal, i2c::SHARED_I2CBUS};
use hal::{fugit::RateExtU32, gpio::Pins, pac};

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

    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        pins.gpio0
            .reconfigure::<hal::gpio::FunctionI2c, hal::gpio::PullUp>(),
        pins.gpio1
            .reconfigure::<hal::gpio::FunctionI2c, hal::gpio::PullUp>(),
        10.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    critical_section::with(|cs| {
        SHARED_I2CBUS.borrow(cs).replace(Some(i2c));
    });
    loop {}
}
