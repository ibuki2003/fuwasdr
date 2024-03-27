use crate::dsp::DSPComplex;

pub fn demod_am(buf: &mut [DSPComplex]) {
    for b in buf {
        *b = b.fast_abs().into();
    }
}
