use core::cell::Cell;

use crate::{codec::Tx, dsp::DSPComplex, sdr::shift::Shifter};
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
        self.fifo.write_blocking(0x8000_0000 | freq as u32);
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
    let buf2 = cortex_m::singleton!(: [DSPComplex; Shifter::OUTPUT_SIZE] = [DSPComplex::zero(); Shifter::OUTPUT_SIZE]).unwrap();

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

    loop {
        let p = fifo.read_blocking();

        if p & 0x8000_0000 != 0 {
            // read command
            let p = ((p << 1) as i32) >> 1; // sign extend

            // set freq is the only command now
            shifter.set_freq(-p);
            continue;
        }

        let t = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();

        // read buffer
        let buffer = unsafe { &*(p as *mut DemodBuffer) };

        // copy at first
        buf.copy_from_slice(buffer);

        shifter.apply(buf, buf2);

        for (i, x) in buf2.iter().enumerate() {
            let v = x.norm().0 as u16;
            for j in 0..4 {
                dmabuf.get_mut()[i * 4 + j] = ((v as u32) << 16) | v as u32;
            }
        }

        let (dma, buf, tx) = tfr.wait();
        let buff = dmabuf.replace(buf);
        tfr = Config::new(dma, buff, tx).start();

        let t2 = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();
        info!("demod time: {} us", t2.wrapping_sub(t));
    }
}
