use core::cell::Cell;

use crate::{hal, codec};
use pac::interrupt;
use critical_section::Mutex;
use hal::{dma::{self, CH0, SingleChannel, Channel}, pac};


pub const DMABUF_LEN: usize = 192*2;
pub static mut DMABUF: [u32; DMABUF_LEN] = [0; DMABUF_LEN];
pub static mut DMA_IDX: usize = 0; // no mutex needed!
pub const DMA_CHUNK_LEN: usize = 64;
pub static mut DMA_TFR: Option<
    dma::single_buffer::Transfer<dma::Channel<dma::CH0>, codec::Rx, &mut [u32]>,
> = None;

pub static FFT_READY: Mutex<Cell<bool>> = Mutex::new(Cell::new(false));

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

pub fn init(mut dmach: Channel<CH0>, rx: codec::Rx) {
    dmach.enable_irq0();
    unsafe {
        defmt::debug_assert!(DMA_IDX == 0);
        let buf = &mut DMABUF[..DMA_CHUNK_LEN];
        let tfr = hal::dma::single_buffer::Config::new(dmach, rx, buf).start();
        DMA_TFR.replace(tfr);

        pac::NVIC::unmask(pac::Interrupt::DMA_IRQ_0);
    }
}
