use auto_ops::impl_op_ex;

const FIXED_POINT: i16 = 14; // available range: [-2, 2)

#[derive(Clone, Copy)]
pub struct DSPNum (pub i16);

impl_op_ex!(+ |a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum(a.0 + b.0) });
impl_op_ex!(- |a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum(a.0 - b.0) });
impl_op_ex!(* |a: &DSPNum, b: &DSPNum| -> DSPNum { DSPNum((a.0 * b.0) >> FIXED_POINT) });

impl_op_ex!(- |a: &DSPNum| -> DSPNum { DSPNum(-a.0) });

impl_op_ex!(+= |a: &mut DSPNum, b: DSPNum| { a.0 += b.0 });
impl_op_ex!(-= |a: &mut DSPNum, b: DSPNum| { a.0 -= b.0 });
impl_op_ex!(*= |a: &mut DSPNum, b: DSPNum| { a.0 = (a.0 * b.0) >> FIXED_POINT });
