use auto_ops::impl_op_ex;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct DSPNum(pub i16);

impl DSPNum {
    pub const FIXED_POINT: i16 = 14; // available range: [-2, 2)
}

impl_op_ex!(+ |a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum(a.0 + b.0) });
impl_op_ex!(-|a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum(a.0 - b.0) });
impl_op_ex!(*|a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum(unshift_fpmul(a.0 as i32 * b.0 as i32)) });

impl_op_ex!(-|a: &DSPNum| -> DSPNum { DSPNum(-a.0) });

impl_op_ex!(+= |a: &mut DSPNum, b: DSPNum| { a.0 += b.0 });
impl_op_ex!(-= |a: &mut DSPNum, b: DSPNum| { a.0 -= b.0 });
impl_op_ex!(*= |a: &mut DSPNum, b: DSPNum| { a.0 = unshift_fpmul(a.0 as i32 * b.0 as i32); });

impl DSPNum {
    pub fn abs(&self) -> DSPNum {
        DSPNum(self.0.abs())
    }

    pub fn sqrt(&self) -> DSPNum {
        let b = (self.0 as u32) * (DSPNum::FIXED_POINT as u32);

        let mut result: u32 = 0;
        let mut shift = 30;
        while shift > 0 {
            result <<= 1;
            let large_cand = result | 1;
            if large_cand * large_cand <= b >> shift {
                result = large_cand;
            }
            shift -= 2;
        }
        DSPNum(result as i16)
    }
}

// get rid of extra "shift" after multiplying inner integer
#[inline]
pub fn unshift_fpmul(a: i32) -> i16 {
    ((a + (1 << (DSPNum::FIXED_POINT - 1))) >> DSPNum::FIXED_POINT) as i16
}
