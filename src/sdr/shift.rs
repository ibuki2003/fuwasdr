use crate::dsp::DSPComplex;

use super::DS_RATIO;

// freq shift and down sample signal
pub struct Shifter {
    phase: u32,
    freq: i32,
    omega: i32,

    rot_buf_a: [DSPComplex; Self::OUTPUT_SIZE],
    rot_buf_b: [DSPComplex; DS_RATIO],
}

impl Shifter {
    pub const INPUT_SIZE: usize = 192;
    pub const OUTPUT_SIZE: usize = Self::INPUT_SIZE / DS_RATIO;

    pub fn new() -> Self {
        Shifter {
            phase: 0,
            freq: 0,
            omega: 0,
            rot_buf_a: [DSPComplex::zero(); Self::OUTPUT_SIZE],
            rot_buf_b: [DSPComplex::zero(); DS_RATIO],
        }
    }

    pub fn set_freq(&mut self, freq: i32) {
        self.freq = freq;
        // self.omega = (freq << 18) / SAMPLE_RATE as i32;
        self.omega = freq * 512 / 375;

        // NOTE: size is now small so naive approach is fine
        for (i, x) in self.rot_buf_a.iter_mut().enumerate() {
            *x = DSPComplex::expi((i as i32) * self.omega * DS_RATIO as i32);
        }
        for (i, x) in self.rot_buf_b.iter_mut().enumerate() {
            *x = DSPComplex::expi((i as i32) * self.omega);
            // DS_RATIO = 4
            *x = *x >> 2;
        }
    }

    pub fn apply(
        &mut self,
        input: &[DSPComplex; Self::INPUT_SIZE],
        output: &mut [DSPComplex; Self::OUTPUT_SIZE],
    ) {
        let p = DSPComplex::expi(self.phase as i32);
        let mut k = 0;
        for (o, a) in output.iter_mut().zip(self.rot_buf_a.iter()) {
            let mut acc = DSPComplex::zero();

            for j in 0..DS_RATIO {
                acc += input[k] * self.rot_buf_b[j];
                k += 1;
            }
            acc *= p;
            acc *= *a;
            *o = acc;
        }

        self.phase = self
            .phase
            .wrapping_add_signed(self.omega * Self::INPUT_SIZE as i32);
    }
}

impl Default for Shifter {
    fn default() -> Self {
        Self::new()
    }
}
