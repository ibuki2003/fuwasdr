#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use rp2040_hal as hal;
use rp_pico as bsp;

use embedded_hal::digital::v2::OutputPin;

use hal::{
    pac,
    watchdog::Watchdog,
    gpio::{Pins, Pin, self},
    Clock,
};

use bsp::{entry, hal::Sio};

#[entry]
fn main() -> ! {
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
        &mut watchdog
    ).ok().unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    let mut led_pin = pins.gpio25.into_push_pull_output();

    loop {
        info!("on!");
        led_pin.set_high().unwrap();
        delay.delay_ms(1000);

        info!("off!");
        led_pin.set_low().unwrap();
        delay.delay_ms(1000);
    }
}
