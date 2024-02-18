// FFT task uses core1

use crate::dsp::fft::{fft, FFTBuffer};
use rp2040_hal::{
    multicore::{self, Multicore, Stack},
    pac,
    sio::SioFifo,
    Sio,
};

static mut CORE1_STACK: Stack<1024> = Stack::new();

// multi-core powered FFT task
pub struct SpectrumTask {
    fifo: SioFifo,
}

impl SpectrumTask {
    pub fn new(
        psm: &mut pac::PSM,
        ppb: &mut pac::PPB,
        mut fifo: SioFifo,
    ) -> Result<Self, multicore::Error> {
        let mut mc = Multicore::new(psm, ppb, &mut fifo);
        let cores = mc.cores();
        let core1 = &mut cores[1];
        core1.spawn(unsafe { &mut CORE1_STACK.mem }, core1_task)?;
        Ok(Self { fifo })
    }

    pub fn run_fft(&mut self, buffer: &'static mut FFTBuffer) {
        let p = buffer as *const FFTBuffer as *mut u8 as u32;
        self.fifo.write_blocking(p);
    }

    pub fn get_result(&mut self) -> Option<&'static mut FFTBuffer> {
        self.fifo
            .read()
            .map(|p| unsafe { &mut *(p as *mut FFTBuffer) })
    }
}

fn core1_task() {
    let pac = unsafe { pac::Peripherals::steal() };
    // let core = unsafe { pac::CorePeripherals::steal() };

    let mut sio = Sio::new(pac.SIO);

    loop {
        let p = sio.fifo.read_blocking();
        defmt::info!("core1: got buffer @ {=u32:x}", p);
        let buffer = unsafe { &mut *(p as *mut FFTBuffer) };
        fft(buffer);
        defmt::info!("core1: done");
        sio.fifo.write_blocking(p);
    }
}
