use defmt::*;
use hal::{fugit::RateExtU32, gpio::Pins, pac, Clock, Sio, Timer, Watchdog};

use crate::{hal, i2c::SHARED_I2CBUS};

const USBBUF_LEN: usize = 192;
static mut USBBUF0: [u8; USBBUF_LEN * 4 * 2] = [0; USBBUF_LEN * 4 * 2];

pub fn main() -> ! {
    info!("Hello, world!");

    // init peripherals
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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // init shared i2c
    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        pins.gpio0
            .reconfigure::<hal::gpio::FunctionI2c, hal::gpio::PullUp>(),
        pins.gpio1
            .reconfigure::<hal::gpio::FunctionI2c, hal::gpio::PullUp>(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    critical_section::with(|cs| {
        SHARED_I2CBUS.borrow(cs).replace(Some(i2c));
    });

    info!("ready");

    delay.delay_ms(1000u32);

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let mut clockctl = crate::clockctl::ClockCtl::new(timer.alarm_0().unwrap());
    clockctl
        .init()
        .unwrap_or_else(|e| info!("Failed to initialize clockctl: {}", e));
    clockctl
        .tune(81300.kHz())
        .unwrap_or_else(|e| info!("Failed to tune: {}", e));

    crate::control::init(pins.gpio22.reconfigure(), pins.gpio26.reconfigure());

    let mut codec = crate::codec::Codec::new(
        pins.gpio2,
        pins.gpio3,
        pins.gpio4,
        pins.gpio5,
        pins.gpio6,
        pins.gpio7,
        pins.gpio8,
        pins.gpio9,
        pac.PIO0,
        &mut pac.RESETS,
    );
    codec.init();
    info!("codec init");

    let mut display = crate::display::Manager::new(crate::display::LcdDisplay::new(
        pac.SPI0,
        pins.gpio14,
        pins.gpio15,
        pins.gpio16,
        pins.gpio17,
        pins.gpio18,
        pins.gpio19,
        pins.gpio20,
        pins.gpio21,
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    ));

    display.init();

    display.draw_text(b"Hello, world!", 0, 0);

    let usb_bus = usb_device::class_prelude::UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut usb_audio = usbd_audio::AudioClassBuilder::new()
        .input(
            usbd_audio::StreamConfig::new_discrete(
                usbd_audio::Format::S16le,
                2,
                &[192000],
                usbd_audio::TerminalType::InMicrophone,
            )
            .unwrap(),
        )
        .build(&usb_bus)
        .unwrap();

    let mut usb_dev = usb_device::prelude::UsbDeviceBuilder::new(
        &usb_bus,
        usb_device::prelude::UsbVidPid(0x16c0, 0x27dd),
    )
    .max_packet_size_0(64)
    .manufacturer("Fake company")
    .product("Audio")
    .serial_number("TEST")
    // .device_class(0xEF)
    .build();

    let mut usb_buf_write = unsafe { &mut USBBUF0[0..USBBUF_LEN * 4] };
    let mut usb_buf_read = unsafe { &mut USBBUF0[USBBUF_LEN * 4..USBBUF_LEN * 4 * 2] };
    let mut usb_buf_idx = 0;

    let mut ctr = 0;

    let mut t = timer.get_counter_low();

    // codec.set_agc_target(7);
    codec.set_agc_target(7);
    loop {
        // sound out
        // while !codec.get_i2s_tx().is_full() {
        //     let v = &SINE_TBL[tx_idx];
        //     let vv = (*v).unsigned_abs() as u32 + 32768;
        //     let vv = vv | (vv << 16);
        //     if codec.get_i2s_tx().write(vv) {
        //         tx_idx += 1;
        //         if tx_idx >= 48 {
        //             tx_idx = 0;
        //         }
        //     }
        // }

        // signal in
        {
            while let Some(v) = codec.read_sample() {
                usb_buf_write[usb_buf_idx..usb_buf_idx + 2].copy_from_slice(&v.0.to_le_bytes());
                usb_buf_write[usb_buf_idx + 2..usb_buf_idx + 4].copy_from_slice(&v.1.to_le_bytes());
                usb_buf_idx += 4;

                if usb_buf_idx >= USBBUF_LEN * 4 {
                    core::mem::swap(&mut usb_buf_write, &mut usb_buf_read);
                    usb_buf_idx = 0;
                }
            }
        }

        // control
        let n = crate::control::pop_count();
        if n != 0 {
            let f = clockctl.get_current_freq();
            let f = f.to_Hz().wrapping_add(n as u32 * 100000);
            clockctl
                .tune(hal::fugit::HertzU32::Hz(f))
                .unwrap_or_else(|e| info!("Failed to tune: {}", e));
            info!("tune to {} kHz", f);

            let mut buf = [0u8; 9];

            let mut f = f;
            for i in (0..9).rev() {
                buf[i] = (f % 10) as u8 + b'0';
                f /= 10;
                if f == 0 {
                    break;
                }
            }

            display.draw_text(&buf, 0, 32);
        }

        // stat log
        let tt = timer.get_counter_low();
        if tt.wrapping_sub(t) > 1_000_000 {
            info!("AGC status: {}", codec.get_agc_gain());
            t = tt;

            info!("poll count: {}", ctr);
            ctr = 0;
        }

        // usb
        if usb_dev.poll(&mut [&mut usb_audio]) {
            // info!("poll");
            ctr += 1;
            usb_audio.write(usb_buf_read).ok();
        }
    }
}
