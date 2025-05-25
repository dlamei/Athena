use std::{cmp, fmt, ops};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i8)]
pub enum Sign {
    #[default]
    Plus = 1,
    Minus = -1,
}

impl Sign {
    #[inline]
    pub const fn flip(self) -> Self {
        match self {
            Sign::Plus => Sign::Minus,
            Sign::Minus => Sign::Plus,
        }
    }

    #[inline]
    pub const fn as_i8(self) -> i8 {
        self as i8
    }

    #[inline]
    pub const fn is_plus(self) -> bool {
        self.as_i8() == Sign::Plus.as_i8()
    }

    #[inline]
    pub const fn is_minus(self) -> bool {
        self.as_i8() == Sign::Minus.as_i8()
    }

    #[inline]
    pub const fn fmt_prefix(&self) -> &'static str {
        match self {
            Sign::Plus => "",
            Sign::Minus => "-",
        }
    }
}

impl ops::Mul for Sign {
    type Output = Sign;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Sign::Minus, Sign::Plus) | (Sign::Plus, Sign::Minus) => Sign::Minus,
            _ => Sign::Plus
        }
    }
}

impl ops::MulAssign for Sign {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

impl ops::Neg for Sign {
    type Output = Sign;

    fn neg(self) -> Self::Output {
        Sign::Minus * self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Real {
    Zero,
    U32(Sign, u32),
}

impl Ord for Real {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        use Real as R;
        match (self, other) {
            (R::Zero, R::Zero) => cmp::Ordering::Equal,
            (R::Zero, R::U32(s, _)) => 0.cmp(&s.as_i8()),
            (R::U32(s1, u1), R::U32(s2, u2)) => {
                if s1 != s2 {
                    s1.cmp(s2)
                } else {
                    u1.cmp(u2)
                }
            }
            (R::U32(s, _), R::Zero) => s.as_i8().cmp(&0),
        }
    }
}
impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Real {


    #[inline]
    pub const fn to_expr(self) -> crate::Expr {
        match self {
            Real::Zero => crate::Expr::u32(0),
            Real::U32(Sign::Minus, u) => crate::Expr::Minus(crate::expr::Atom::U32(u)),
            Real::U32(Sign::Plus, u) => crate::Expr::u32(u),
        }
    }


    #[inline]
    pub const fn u32_with_sign(s: Sign, u: u32) -> Self {
        if u == 0 {
            Real::Zero
        } else {
            Real::U32(s, u)
        }
    }


    #[inline]
    pub const fn u32(u: u32) -> Self {
        if u == 0 {
            Self::Zero
        } else {
            Real::U32(Sign::Plus, u)
        }
    }

    #[inline]
    pub const fn i32(i: i32) -> Self {
        if i == 0 {
            return Self::Zero
        }
        let mut r = Self::u32(i.unsigned_abs());
        if i < 0 {
            r.flip_sign();
            r
        } else {
            r
        }
    }

    #[inline]
    pub const fn flip_sign(&mut self) {
        match self {
            Real::Zero => (),
            Real::U32(sign, u) => *self = Real::U32(sign.flip(), *u),
        }
    }

    /// `0` is neither negative nor positive
    #[inline]
    pub const fn is_negative(&self) -> bool { 
        match self {
            Real::Zero => false,
            Real::U32(sign, _) => sign.is_minus(),
        }
    }

    /// `0` is neither negative nor positive
    #[inline]
    pub const fn is_positive(&self) -> bool { 
        match self {
            Real::Zero => false,
            Real::U32(sign, _) => sign.is_plus(),
        }
    }

    #[inline]
    pub const fn is_zero(&self) -> bool {
        matches!(self, Real::Zero)
    }

    #[inline]
    pub fn abs(&self) -> Self {
        match self {
            Real::Zero => Real::Zero,
            Real::U32(_, u) => Real::U32(Sign::Plus, *u),
        }
    }

    #[inline]
    pub fn sign(&self) -> Option<Sign> {
        match self {
            Real::Zero => None,
            Real::U32(sign, _) => Some(*sign),
        }
    }

    #[inline]
    pub fn pow(self, exp: Real) -> Self {
        match (self, exp) {
            (Real::Zero, Real::Zero) => panic!("0^0"),
            (Real::Zero, Real::U32(sign, _)) => {
                if sign.is_minus() {
                    panic!("div by 0")
                } else {
                    Real::Zero
                }
            },
            (Real::U32(_, _), Real::Zero) => {
                Real::u32(1)
            },
            (Real::U32(_, _), Real::U32(Sign::Minus, _)) => {
                panic!("fractions not supported")
            }
            (Real::U32(mut sb, b), Real::U32(Sign::Plus, e)) => {
                if e % 2 == 0 {
                    sb = Sign::Plus;
                }
                Real::U32(sb, b.pow(e))
            },
        }
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Real::Zero => write!(f, "0"),
            Real::U32(sign, u) => write!(f, "{}{u}", sign.fmt_prefix()),
        }
    }
}

impl ops::Neg for Real {
    type Output = Real;

    fn neg(self) -> Self::Output {
        match self {
            Real::U32(sign, u) => Real::U32(-sign, u),
            Real::Zero => Real::Zero,
        }
    }
}

impl ops::Add for Real {
    type Output = Real;

    fn add(self, rhs: Self) -> Self::Output {
        use Real as R;

        match (self, rhs) {
            (_, R::Zero) => self,
            (R::Zero, _) => rhs,
            (R::U32(sa, a), R::U32(sb, b)) => match (sa, sb) {
                (Sign::Plus, Sign::Plus) => R::U32(Sign::Plus, a + b),
                (Sign::Minus, Sign::Minus) => R::U32(Sign::Minus, a + b),
                (Sign::Plus, Sign::Minus) => {
                    if a >= b { R::U32(Sign::Plus, a - b) } else { R::U32(Sign::Minus, b - a) }
                }
                (Sign::Minus, Sign::Plus) => {
                    if b >= a { R::U32(Sign::Plus, b - a) } else { R::U32(Sign::Minus, a - b) }
                }
            }
        }
    }
}

impl ops::Sub for Real {
    type Output = Real;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}
impl ops::SubAssign for Real {
    fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
}

impl ops::Mul for Real {
    type Output = Real;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Real::Zero, _) | (_, Real::Zero) => Real::Zero,
            (Real::U32(s1, a), Real::U32(s2, b)) => Real::U32(s1 * s2, a * b),
        }
    }
}

impl ops::MulAssign for Real {
    fn mul_assign(&mut self, rhs: Self) { *self = *self * rhs; }
}

impl ops::Div for Real {
    type Output = Real;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Real::Zero) => panic!("Division by zero"),
            (Real::Zero, _) => Real::Zero,
            (Real::U32(s1, u1), Real::U32(s2, u2)) => {
                if u1.rem_euclid(u2) != 0 {
                    panic!("fractions not supported");
                }
                Real::U32(s1 * s2, u1 / u2)
            }
        }
    }
}
impl ops::DivAssign for Real {
    fn div_assign(&mut self, rhs: Self) { *self = *self / rhs; }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(v: i32) -> Real {
        Real::i32(v)
    }

    #[test]
    fn cmp() {
        assert!(r(3) > r(0));
        assert!(r(-3) < r(0));
        assert!(Real::u32(0) == Real::i32(0));
    }

    #[test]
    fn add_sub() {
        assert_eq!(r(3) + r(-5), r(-2));
        assert_eq!(r(5) - r(3), r(2));
    }

    #[test]
    fn mul_div() {
        assert_eq!(r(-2) * r(3), r(-6));
        assert_eq!(r(8) / r(2), r(4));
    }

    #[test]
    fn pow() {
        assert_eq!(r(-2).pow(r(3)), r(-8));
        assert_eq!(r(-2).pow(r(2)), r(4));
    }

    #[test]
    fn basic() {
        assert!(r(0).is_zero());
        assert!(!r(0).is_positive());
    }
}

