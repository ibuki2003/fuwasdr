use crate::{
    board, codec, display,
    dsp::{self, DSPComplex},
    hal,
    i2c::SHARED_I2CBUS,
};
use defmt::*;
use hal::{
    dma::DMAExt,
    fugit::RateExtU32,
    pac::{self},
    Clock, Sio, Timer, Watchdog,
};

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
    let pins = board::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    crate::dsp::fft::make_sequential_expi();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // init shared i2c
    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        pins.i2c_sda.reconfigure(),
        pins.i2c_scl.reconfigure(),
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

    crate::control::init(
        pins.rotary_a.reconfigure(),
        pins.rotary_b.reconfigure(),
        pins.button_1.reconfigure(),
        pins.button_2.reconfigure(),
    );

    let mut codec = codec::Codec::new(
        pins.codec_mclk.reconfigure(),
        pins.codec_bclk.reconfigure(),
        pins.codec_wclk.reconfigure(),
        pins.codec_mfp1.reconfigure(),
        pins.codec_mfp2.reconfigure(),
        pins.codec_mfp3.reconfigure(),
        pins.codec_mfp4.reconfigure(),
        pins.codec_mfp5.reconfigure(),
        pac.PIO0,
        &mut pac.RESETS,
    );
    codec.init();

    let dma = pac.DMA.split(&mut pac.RESETS);
    super::dma::init(dma.ch0, codec.take_rx().unwrap());

    let mut display = display::Manager::new(display::LcdDisplay::new(
        pac.SPI0,
        pins.lcd_reset.reconfigure(),
        pins.lcd_touchirq.reconfigure(),
        pins.lcd_miso.reconfigure(),
        pins.lcd_touchcs.reconfigure(),
        pins.lcd_sck.reconfigure(),
        pins.lcd_mosi.reconfigure(),
        pins.lcd_dispcs.reconfigure(),
        pins.lcd_dcrs.reconfigure(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    ));
    display.init();

    let usb_bus = usb_device::bus::UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    static mut USBBUS: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None;
    unsafe {
        USBBUS.replace(usb_bus);
    }

    super::usb::UsbDev::init(unsafe { USBBUS.as_ref().unwrap() });

    const FFTBUF_LEN: usize = 256;
    let mut fft_buf = [DSPComplex::zero(); FFTBUF_LEN];

    let mut t = timer.get_counter_low();

    let mut gain = 0;
    codec.set_adc_gain(gain);

    const TS_TBL: [u32; 9] = [
        1,
        10,
        100,
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
    ];
    let mut cursor = 0;
    const CURSOR_MOD: u8 = TS_TBL.len() as u8 + 1;

    display.draw_freq(clockctl.get_current_freq().to_Hz());
    display.draw_cursor(cursor);

    // main loop
    loop {
        {
            let fft_ready = critical_section::with(|cs| {
                // if true, set false and return
                crate::core::dma::FFT_READY.borrow(cs).replace(false)
            });

            if fft_ready {
                fft_buf.copy_from_slice(unsafe {
                    core::slice::from_raw_parts(
                        crate::core::dma::DMABUF.as_ptr() as *const DSPComplex,
                        256,
                    )
                });
                dsp::fft::fft(&mut fft_buf);
                display.draw_spectrum(&fft_buf);
            }
        }

        // control
        let (rot, btn) = crate::control::fetch_inputs();
        if btn != 0 {
            if btn & 1 != 0 {
                cursor += 1;
            }
            if btn & 2 != 0 {
                // cursor -= 1;
                cursor += CURSOR_MOD - 1;
            }
            // cursor = cursor.clamp(0, 9);
            cursor %= CURSOR_MOD;
            display.draw_cursor(cursor);
        }
        if rot != 0 {
            match cursor {
                0..=8 => {
                    // tune
                    let f = clockctl.get_current_freq();
                    let tune_step = TS_TBL[cursor as usize].max(clockctl.get_tune_step() as u32);
                    let f = f.to_Hz().wrapping_add(rot as u32 * tune_step);
                    match clockctl.tune(hal::fugit::HertzU32::Hz(f)) {
                        Err(e) => info!("Failed to tune: {}", e),
                        Ok(_) => display.draw_freq(f),
                    }
                }
                9 => {
                    // agc
                    gain = (gain as i32 + rot).clamp(0, 95) as u8;
                    codec.set_adc_gain(gain);
                    let mut buf = [0u8; 2];
                    buf[0] = b'0' + gain / 10;
                    buf[1] = b'0' + gain % 10;
                    display.draw_text(&buf, 280, 10);
                }
                _ => core::unreachable!(),
            }
        }

        // stat log
        let tt = timer.get_counter_low();
        if tt.wrapping_sub(t) > 1_000_000 {
            info!("AGC status: {}", codec.get_agc_gain());
            t = tt;
        }
    }
}
