use crate::SAMPLE_RATE;

pub const DS_RATIO: usize = 4;
pub const DS_RATE: usize = SAMPLE_RATE / DS_RATIO;

pub mod shift;
