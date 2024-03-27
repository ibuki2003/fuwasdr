mod am;
mod fm;
pub use am::demod_am;
pub use fm::demod_fm;

#[derive(Copy, Clone)]
pub enum DemodMethod {
    AM,
    FM,
}
impl DemodMethod {
    pub const METHOD_COUNT: u8 = 2;
    /// # Safety
    ///
    /// value should be in range. otherwise causes UB
    pub unsafe fn from_u8(value: u8) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}
