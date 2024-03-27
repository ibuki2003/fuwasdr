use crate::dsp::DSPComplex;

static mut LAST: i32 = 0;
pub fn demod_fm(buf: &mut [DSPComplex]) {
    let mut last = unsafe { LAST };
    for b in buf {
        let p = b.phase() >> 2; // 18 -> 16
        let diff = p.wrapping_sub(last) as i16;
        last = p;
        *b = DSPComplex::from_i16(diff, 0);
    }

    unsafe { LAST = last };
}
