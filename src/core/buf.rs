use core::{cell::Cell, mem::MaybeUninit};

pub const CHUNK_SIZE: usize = 256;
type Chunk = &'static mut [u8; CHUNK_SIZE];

static mut RAW_BUF1: MaybeUninit<[u8; CHUNK_SIZE]> = MaybeUninit::uninit();
static mut RAW_BUF2: MaybeUninit<[u8; CHUNK_SIZE]> = MaybeUninit::uninit();

pub static mut CUR_CHUNK: Cell<Option<&'static mut [u8; CHUNK_SIZE]>> = Cell::new(None);

pub fn init_double_buffer() -> Chunk {
    unsafe {
        let buf1 = &mut *RAW_BUF1.as_mut_ptr();
        let buf2 = &mut *RAW_BUF2.as_mut_ptr();
        CUR_CHUNK.set(Some(buf1));
        buf2
    }
}
