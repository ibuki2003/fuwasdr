use crate::dsp::DSPComplex;

const FFT_SIZE: usize = 256;
const FFT_SIZE_LOG2: usize = 8;

pub type FFTBuffer = [DSPComplex; 256];

static mut FFT_OMEGAS: [DSPComplex; 128] = [DSPComplex::zero(); 128];

pub fn make_sequential_expi() {
    let omegas = unsafe { &mut FFT_OMEGAS };
    DSPComplex::make_sequential_expi(omegas);
}

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

        for i in 0..(1 << j) {
            if i & b != 0 {
                continue;
            }

            let w = unsafe { *FFT_OMEGAS.get_unchecked(i << (7 - j)) };

            let i2 = i | b;
            for i0 in (0..FFT_SIZE).step_by(1 << (j + 1)) {
                // div by 2 to normalize
                let a = unsafe { *arr.get_unchecked(i0 | i) } >> 1;
                let b = (unsafe { arr.get_unchecked(i0 | i2) } * w) >> 1;
                unsafe {
                    *arr.get_unchecked_mut(i0 | i) = a + b;
                    *arr.get_unchecked_mut(i0 | i2) = a - b;
                }
            }
        }
    }

    // fix frequency order
    for i in 0..128 {
        arr.swap(i, i | 128);
    }
}
