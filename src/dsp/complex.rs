use auto_ops::impl_op_ex;
use super::number::DSPNum;

#[derive(Clone, Copy)]
pub struct DSPComplex {
    pub re: DSPNum,
    pub im: DSPNum,
}

impl_op_ex!(+ |a: &DSPComplex, b: &DSPComplex| -> DSPComplex { DSPComplex { re: a.re + b.re, im: a.im + b.im } });
impl_op_ex!(- |a: &DSPComplex, b: &DSPComplex| -> DSPComplex { DSPComplex { re: a.re - b.re, im: a.im - b.im } });
impl_op_ex!(* |a: &DSPComplex, b: &DSPComplex| -> DSPComplex {
    DSPComplex {
        re: a.re * b.re - a.im * b.im,
        im: a.re * b.im + a.im * b.re,
    }
});

impl_op_ex!(- |a: &DSPComplex| -> DSPComplex { DSPComplex { re: -a.re, im: -a.im } });

impl_op_ex!(+= |a: &mut DSPComplex, b: DSPComplex| { a.re += b.re; a.im += b.im });
impl_op_ex!(-= |a: &mut DSPComplex, b: DSPComplex| { a.re -= b.re; a.im -= b.im });
impl_op_ex!(*= |a: &mut DSPComplex, b: DSPComplex| { *a = *a * b });
