use crate::{
    codec::Tx,
    dsp::DSPComplex,
    sdr::{
        demod::{demod_am, demod_fm, DemodMethod},
        shift::Shifter,
    },
};
use core::cell::Cell;
use defmt::info;
use rp2040_hal::{
    dma::single_buffer::Config,
    multicore::{self, Multicore, Stack},
    pac,
    sio::SioFifo,
    Sio,
};

pub type DmaCh = rp2040_hal::dma::CH1;
pub type Dma = rp2040_hal::dma::Channel<DmaCh>;

static mut CORE1_STACK: Stack<4096> = Stack::new();

pub const DEMOD_BUF_SIZE: usize = Shifter::INPUT_SIZE;
pub type DemodBuffer = [DSPComplex; DEMOD_BUF_SIZE];

// multi-core powered Demodulation
pub struct DemodTask {
    fifo: SioFifo,
}

impl DemodTask {
    pub fn new(
        psm: &mut pac::PSM,
        ppb: &mut pac::PPB,
        mut fifo: SioFifo,
        tx: Tx,
        dma: Dma,
    ) -> Result<Self, multicore::Error> {
        let mut mc = Multicore::new(psm, ppb, &mut fifo);
        let cores = mc.cores();
        let core1 = &mut cores[1];
        core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
            core1_task(tx, dma);
        })?;
        Ok(Self { fifo })
    }

    // core1 should copy the buffer to its own memory as soon as possible
    pub fn send_buffer(&mut self, buffer: &'static DemodBuffer) {
        let p = buffer as *const DemodBuffer as *const u8 as u32;
        debug_assert!(p & 0x8000_0000 == 0);
        self.fifo.write_blocking(p);
    }

    pub fn set_freq(&mut self, freq: i32) {
        self.fifo
            .write_blocking(0x8000_0000 | (freq as u32 & 0xffffff));
    }

    pub fn set_method(&mut self, method: DemodMethod) {
        self.fifo.write_blocking(0x8100_0000 | method as u32);
    }
}

fn core1_task(tx: Tx, dma: Dma) {
    let pac = unsafe { pac::Peripherals::steal() };
    // let core = unsafe { pac::CorePeripherals::steal() };

    let sio = Sio::new(pac.SIO);
    let mut fifo = sio.fifo;

    let mut shifter = crate::sdr::shift::Shifter::new();
    shifter.set_freq(0);

    let buf = cortex_m::singleton!(: DemodBuffer = [DSPComplex::zero(); DEMOD_BUF_SIZE]).unwrap();
    let buf_ds = cortex_m::singleton!(: [DSPComplex; Shifter::OUTPUT_SIZE] = [DSPComplex::zero(); Shifter::OUTPUT_SIZE]).unwrap();

    // to pass 192kHz sampled data
    let mut dmabuf = Cell::new(
        cortex_m::singleton!(: [u32; Shifter::INPUT_SIZE] = [0; Shifter::INPUT_SIZE]).unwrap(),
    );

    // double buffering; start first transfer for seamless operation
    let mut tfr = Config::new(
        dma,
        cortex_m::singleton!(: [u32; Shifter::INPUT_SIZE] = [0; Shifter::INPUT_SIZE]).unwrap(),
        tx,
    )
    .start();

    let mut method = DemodMethod::AM;

    loop {
        let p = fifo.read_blocking();

        if p & 0x8000_0000 != 0 {
            // read command
            let c = p >> 24;
            match c {
                0x80 => {
                    // tune
                    let f = ((p << 8) as i32) >> 8; // sign extend

                    // set freq is the only command now
                    shifter.set_freq(-f);
                }
                0x81 => {
                    // demodulation method
                    method = unsafe { DemodMethod::from_u8(p as u8) };
                }
                _ => {}
            }
            continue;
        }

        let t = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();

        // read buffer
        let buffer = unsafe { &*(p as *mut DemodBuffer) };

        // copy at first
        buf.copy_from_slice(buffer);

        if !matches!(method, DemodMethod::FM) {
            shifter.apply(buf, buf_ds);
            let t2 = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();
            info!("shift time: {} us", t2.wrapping_sub(t));
        }

        // here demod_**() process into buf2
        match method {
            DemodMethod::AM => {
                demod_am(buf_ds);
            }
            DemodMethod::FM => {
                demod_fm(buf);
                // downsample
                for i in (0..buf.len()).step_by(4) {
                    let mut a: i32 = 0;
                    for j in 0..4 {
                        a += buf[i + j].re.0 as i32;
                    }
                    a /= 4;
                    buf_ds[i >> 2].re.0 = a as i16;
                }
            }
        }

        for (i, x) in buf_ds.iter().enumerate() {
            let v = x.re.0;
            for j in 0..4 {
                dmabuf.get_mut()[i * 4 + j] = ((v as u32) << 16) | v as u32;
            }
        }

        let t2 = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();
        info!("demod time: {} us", t2.wrapping_sub(t));

        let (dma, buf, tx) = tfr.wait();
        let buff = dmabuf.replace(buf);
        tfr = Config::new(dma, buff, tx).start();
    }
}
