use crate::dsp::DSPComplex;

const FFT_SIZE: usize = 256;
const FFT_SIZE_LOG2: usize = 8;

pub type FFTBuffer = [DSPComplex; 256];

/*
apply FFT to arr in-place

result is normalized by 1/256
*/
pub fn fft(arr: &mut FFTBuffer) {
    for i in 0u8..=255 {
        let j = i.reverse_bits();
        if i < j {
            arr.swap(i as usize, j as usize);
        }
    }

    for j in 0..FFT_SIZE_LOG2 {
        let b = 1 << j;
        let omega = DSPComplex::expi(1i32 << (18-(j + 1)));
        let mut w = DSPComplex::one();

        for i in 0..(1 << j) {
            if i & b != 0 { continue; }
            let i2 = i | b;
            for i0 in (0..FFT_SIZE).step_by(1 << (j + 1)) {
                // div by 2 to normalize
                let a = unsafe {arr.get_unchecked(i0 | i)} >> 1;
                let b = (unsafe { arr.get_unchecked(i0 | i2) } * w) >> 1;
                unsafe {
                    *arr.get_unchecked_mut(i0 | i) = a + b;
                    *arr.get_unchecked_mut(i0 | i2) = a - b;
                }
            }
            w *= omega;
        }
    }
}
