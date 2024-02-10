pub fn reduce(mut a: u32, mut b: u32) -> (u32, u32) {
    if a == 0 || b == 0 {
        return (a, b);
    }

    let mut a_s = 1;
    let mut b_s = 1;

    let aa = a.trailing_zeros();
    let bb = b.trailing_zeros();

    if aa < bb {
        a >>= aa;
        b >>= bb;
        b_s <<= bb - aa;
    } else {
        a >>= aa;
        b >>= bb;
        a_s <<= aa - bb;
    }

    if a == b {
        return (a_s, b_s);
    }

    // now a and b is odd

    let mut r = if a < b {
        b -= a;
        let mut r = reduce(a, b);
        r.1 += r.0;
        r
    } else {
        a -= b;
        let mut r = reduce(a, b);
        r.0 += r.1;
        r
    };
    r.1 *= b_s;
    r.0 *= a_s;
    r
}
