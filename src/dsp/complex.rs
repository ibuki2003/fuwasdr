use super::number::unshift_fpmul;
use super::number::DSPNum;
use auto_ops::impl_op_ex;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(4))]
pub struct DSPComplex {
    pub re: DSPNum,
    pub im: DSPNum,
}

impl_op_ex!(+ |a: &DSPComplex, b: &DSPComplex| -> DSPComplex { DSPComplex { re: a.re + b.re, im: a.im + b.im } });
impl_op_ex!(-|a: &DSPComplex, b: &DSPComplex| -> DSPComplex {
    DSPComplex {
        re: a.re - b.re,
        im: a.im - b.im,
    }
});
impl_op_ex!(*|a: &DSPComplex, b: &DSPComplex| -> DSPComplex {
    DSPComplex {
        re: DSPNum(unshift_fpmul(
            a.re.0 as i32 * b.re.0 as i32 - a.im.0 as i32 * b.im.0 as i32,
        )),
        im: DSPNum(unshift_fpmul(
            a.re.0 as i32 * b.im.0 as i32 + a.im.0 as i32 * b.re.0 as i32,
        )),
    }
});

impl_op_ex!(<< |a: &DSPComplex, b: u16| -> DSPComplex {
    DSPComplex {
        re: a.re << b,
        im: a.im << b,
    }
});
impl_op_ex!(>> |a: &DSPComplex, b: u16| -> DSPComplex {
    DSPComplex {
        re: a.re >> b,
        im: a.im >> b,
    }
});

impl_op_ex!(-|a: &DSPComplex| -> DSPComplex {
    DSPComplex {
        re: -a.re,
        im: -a.im,
    }
});

impl_op_ex!(+= |a: &mut DSPComplex, b: DSPComplex| { a.re += b.re; a.im += b.im });
impl_op_ex!(-= |a: &mut DSPComplex, b: DSPComplex| { a.re -= b.re; a.im -= b.im });
impl_op_ex!(*= |a: &mut DSPComplex, b: DSPComplex| { *a = *a * b });

impl DSPComplex {
    pub const fn zero() -> DSPComplex {
        DSPComplex {
            re: DSPNum(0),
            im: DSPNum(0),
        }
    }
    pub const fn one() -> DSPComplex {
        DSPComplex {
            re: DSPNum(1 << DSPNum::FIXED_POINT),
            im: DSPNum(0),
        }
    }
    pub const fn i() -> DSPComplex {
        DSPComplex {
            re: DSPNum(0),
            im: DSPNum(1 << DSPNum::FIXED_POINT),
        }
    }

    pub const fn from_i16(re: i16, im: i16) -> DSPComplex {
        DSPComplex {
            re: DSPNum(re),
            im: DSPNum(im),
        }
    }

    pub const fn conj(&self) -> DSPComplex {
        DSPComplex {
            re: self.re,
            im: DSPNum(-self.im.0),
        }
    }

    // returns (cos(theta * pi / 2 / 2^16) + sin(theta * pi / 2 / 2^16) * i)
    pub fn expi(theta: i32) -> DSPComplex {
        let orthant = (theta >> 16) & 3;
        let theta = (theta & 0xffff) as u16;
        match orthant {
            0 => sincos_(theta),
            1 => {
                let c = sincos_(theta);
                DSPComplex {
                    re: -c.im,
                    im: c.re,
                }
            }
            2 => {
                let c = sincos_(theta);
                DSPComplex {
                    re: -c.re,
                    im: -c.im,
                }
            }
            3 => {
                let c = sincos_(theta);
                DSPComplex {
                    re: c.im,
                    im: -c.re,
                }
            }
            _ => unreachable!(),
        }
    }

    // atan2(y, x) / (pi / 2) * 2^16
    pub fn phase(&self) -> i32 {
        if self.re.0 == 0 && self.im.0 == 0 {
            return 0;
        }
        if self.re.0 >= 0 {
            if self.im.0 >= 0 {
                atan2_(*self) as i32
            } else {
                -(atan2_(DSPComplex {
                    re: self.re,
                    im: -self.im,
                }) as i32)
            }
        } else {
            // re < 0
            if self.im.0 >= 0 {
                (1 << (16 + 1))
                    - (atan2_(DSPComplex {
                        re: -self.re,
                        im: self.im,
                    }) as i32)
            } else {
                (atan2_(DSPComplex {
                    re: -self.re,
                    im: -self.im,
                }) as i32)
                    - (1 << (16 + 1))
            }
        }
    }

    pub fn norm(&self) -> DSPNum {
        let re = self.re.0 as i32;
        let im = self.im.0 as i32;
        DSPNum(((re * re + im * im) >> (DSPNum::FIXED_POINT)) as i16)
    }

    // arr[i] = expi(i/128 * pi).conj()
    pub fn make_sequential_expi(arr: &mut [DSPComplex; 128]) {
        arr[0] = DSPComplex::one();
        arr[64] = DSPComplex::i().conj();
        for i in 0..6 {
            arr[1 << i] = COSSIN_TABLE[5 - i].conj();
        }

        let mut d = 2;
        for i in 3..128 {
            if is_power_of_two(i) {
                d <<= 1;
            } else {
                let j = i & !d;
                arr[i] = arr[j] * arr[d];
            }
        }
    }
}

const fn is_power_of_two(x: usize) -> bool {
    x & (x - 1) == 0
}

const COSSIN_TABLE: [DSPComplex; 16] = [
    DSPComplex::from_i16(11585, 11585),
    DSPComplex::from_i16(15137, 6270),
    DSPComplex::from_i16(16069, 3196),
    DSPComplex::from_i16(16305, 1606),
    DSPComplex::from_i16(16364, 804),
    DSPComplex::from_i16(16379, 402),
    DSPComplex::from_i16(16383, 201),
    DSPComplex::from_i16(16384, 101),
    DSPComplex::from_i16(16384, 50),
    DSPComplex::from_i16(16384, 25),
    DSPComplex::from_i16(16384, 13),
    DSPComplex::from_i16(16384, 6),
    DSPComplex::from_i16(16384, 3),
    DSPComplex::from_i16(16384, 2),
    DSPComplex::from_i16(16384, 1),
    DSPComplex::from_i16(16384, 0),
];

#[inline]
fn sincos_(x: u16) -> DSPComplex {
    let mut sc = DSPComplex::from_i16(1 << DSPNum::FIXED_POINT, 0);

    for i in 0..16 {
        if x & (1 << i) != 0 {
            sc *= COSSIN_TABLE[15 - i];
        }
    }
    sc
}

#[inline]
fn atan2_(xy: DSPComplex) -> u16 {
    let mut angle: u16 = 0;
    let mut sc = DSPComplex::from_i16(1 << DSPNum::FIXED_POINT, 0);

    for i in (0..16).rev() {
        let sc2 = sc * COSSIN_TABLE[15 - i];
        if (sc2.im.0 as i32 * xy.re.0 as i32) < (sc2.re.0 as i32 * xy.im.0 as i32) {
            angle |= 1 << i;
            sc = sc2;
        }
    }
    angle
}
