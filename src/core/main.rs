use super::spectrum;
use crate::{board, codec, display, dsp::DSPComplex, hal, i2c::SHARED_I2CBUS};
use core::cell::Cell;
use critical_section::Mutex;
use defmt::*;
use hal::{
    dma::{self, DMAExt, SingleChannel},
    fugit::RateExtU32,
    pac::{self, interrupt},
    Clock, Sio, Timer, Watchdog,
};

const DMABUF_LEN: usize = 192*2;
static mut DMABUF: [u32; DMABUF_LEN] = [0; DMABUF_LEN];
static mut DMA_IDX: usize = 0; // no mutex needed!
const DMA_CHUNK_LEN: usize = 64;
static mut DMA_TFR: Option<
    dma::single_buffer::Transfer<dma::Channel<dma::CH0>, codec::Rx, &mut [u32]>,
> = None;

static FFT_READY: Mutex<Cell<bool>> = Mutex::new(Cell::new(false));

#[allow(non_snake_case)]
#[interrupt]
fn DMA_IRQ_0() {
    let idx = unsafe { &mut DMA_IDX };

    if !unsafe { DMA_TFR.as_ref().map_or(false, |t| t.is_done()) } {
        defmt::warn!("TFR not done");
        return;
    }

    let tfr = unsafe { DMA_TFR.take().unwrap() };

    let (mut ch, from, _) = tfr.wait();

    // clear flag
    ch.check_irq0();

    *idx += DMA_CHUNK_LEN;
    if *idx >= DMABUF_LEN {
        critical_section::with(|cs| {
            FFT_READY.borrow(cs).set(true);
        });
        *idx = 0;
    }

    let next = unsafe { &mut DMABUF[*idx..*idx + DMA_CHUNK_LEN] };
    unsafe {
        DMA_TFR.replace(hal::dma::single_buffer::Config::new(ch, from, next).start());
    }
}

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

    let mut spectrumtask =
        spectrum::SpectrumTask::new(&mut pac.PSM, &mut pac.PPB, sio.fifo).unwrap();
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

    let mut dma = pac.DMA.split(&mut pac.RESETS);
    // start first transfer
    dma.ch0.enable_irq0();
    unsafe {
        defmt::debug_assert!(DMA_IDX == 0);
        let buf = &mut DMABUF[..DMA_CHUNK_LEN];
        let tfr =
            hal::dma::single_buffer::Config::new(dma.ch0, codec.take_rx().unwrap(), buf).start();
        DMA_TFR.replace(tfr);

        pac::NVIC::unmask(pac::Interrupt::DMA_IRQ_0);
    }

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

    const FFTBUF_LEN: usize = 256;
    static mut FFTBUF: [DSPComplex; FFTBUF_LEN] = [DSPComplex::zero(); FFTBUF_LEN];
    let mut fft_buf = Some(unsafe { &mut FFTBUF });

    const USBBUF_LEN: usize = 192;
    static mut USBBUF: [u8; USBBUF_LEN * 4] = [0; USBBUF_LEN * 4];
    let usb_buf = unsafe { &mut USBBUF };

    let mut t = timer.get_counter_low();

    let mut agc = 6;
    codec.set_agc_target(agc);

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
                FFT_READY.borrow(cs).replace(false)
            });

            if fft_ready {
                if let Some(buf) = fft_buf.take() {
                    buf.copy_from_slice(unsafe {
                        core::slice::from_raw_parts(
                            DMABUF.as_ptr() as *const DSPComplex,
                            256,
                        )
                    });
                    spectrumtask.run_fft(buf);
                }
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
                    agc = (agc as i32 + rot).clamp(0, 7) as u8;
                    codec.set_agc_target(agc);
                    display.draw_text(&[b'0' + agc], 300, 10);
                }
                _ => core::unreachable!(),
            }
        }

        {
            if let Some(v) = spectrumtask.get_result() {
                display.draw_spectrum(v);
                // push back to buffer pool
                fft_buf.replace(v);
            }
        }

        // stat log
        let tt = timer.get_counter_low();
        if tt.wrapping_sub(t) > 1_000_000 {
            info!("AGC status: {}", codec.get_agc_gain());
            t = tt;
        }

        // usb
        if usb_dev.poll(&mut [&mut usb_audio]) {
            let src_idx = if unsafe { DMA_IDX } < USBBUF_LEN { USBBUF_LEN } else { 0 };
            let src = unsafe { &DMABUF[src_idx..src_idx + USBBUF_LEN] };
            unsafe {core::slice::from_raw_parts_mut(usb_buf.as_mut_ptr() as *mut u32, USBBUF_LEN)}
                .copy_from_slice(src);
            usb_audio.write(usb_buf).ok();
        }
    }
}
