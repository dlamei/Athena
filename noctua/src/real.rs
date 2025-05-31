use std::{cmp, fmt, ops};

use num::rational::Ratio;

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
            _ => Sign::Plus,
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

pub fn pow_ratio(mut sb: Sign, mut b: Ratio<u32>, mut se: Sign, e: Ratio<u32>) -> (Real, Option<Real>) {
    if b == Ratio::ZERO && e == Ratio::ZERO {
        panic!("0^0")
    }

    if b == Ratio::ZERO {
        return (Real::u32(0), None)
    }

    if se.is_minus() {
        b = Ratio::new(*b.denom(), *b.numer());
        se = se.flip();
    }

    if e.is_integer() {
        let exp = *e.numer();
        if exp % 2 == 0 {
            sb = Sign::Plus;
        }
        (Real::signed_rational(sb, b.pow(exp as i32)), None)
    } else if e.numer() > e.denom() {
        let (n, d) = (*e.numer(), *e.denom());
        let (quot, rem) = num::integer::div_rem(n, d);
        let exp = quot;
        if exp % 2 == 0 {
            sb = Sign::Plus;
        }
        (Real::signed_rational(sb, b.pow(exp as i32)), Some(Real::u32(rem)))
    } else {
        (Real::signed_rational(sb, b), Some(Real::signed_rational(se, e)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealTyp {
    Zero,
    U32(u32),
    Ratio(Ratio<u32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Real {
    pub(crate) typ: RealTyp,
    pub(crate) sign: Sign,
    // Zero,
    // U32(Sign, u32),
}

impl Ord for Real {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        use RealTyp as R;
        match (self.typ, other.typ) {
            (R::Zero, R::Zero) => cmp::Ordering::Equal,
            (R::Zero, _) => 0.cmp(&other.sign.as_i8()),
            (_, R::Zero) => self.sign.as_i8().cmp(&0),
            (R::U32(u1), R::U32(u2)) => {
                if self.sign != other.sign {
                    self.sign.cmp(&other.sign)
                } else {
                    u1.cmp(&u2)
                }
            }
            (R::U32(u), R::Ratio(r)) => {
                Ratio::new(u, 1).cmp(&r)
            },
            (R::Ratio(r), R::U32(u)) => {
                r.cmp(&Ratio::new(u, 1))
            },
            (R::Ratio(r1), R::Ratio(r2)) => {
                r1.cmp(&r2)
            },
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
    pub const fn signed_u32(s: Sign, u: u32) -> Self {
        // if u == 0 { Real::Zero } else { Real::U32(s, u) }
        let mut r = Real::u32(u);
        r.sign = s;
        r
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            typ: RealTyp::Zero,
            sign: Sign::Plus,
        }
    }

    #[inline]
    pub const fn u32(u: u32) -> Self {
        if u == 0 {
            Real::zero()
        } else {
            Real {
                typ: RealTyp::U32(u),
                sign: Sign::Plus,
            }
        }
    }

    #[inline]
    pub const fn i32(i: i32) -> Self {
        if i == 0 {
            return Self::zero();
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
    pub fn signed_rational(sign: Sign, r: Ratio<u32>) -> Self {
        let (n, d) = (*r.numer(), *r.denom());
        if n == 0 && d == 0 {
            panic!("0 / 0");
        } else if n == 0 {
            Real::zero()
        } else if d == 1 {
            Real::signed_u32(sign, n)
        } else {
            Self {
                typ: RealTyp::Ratio(r),
                sign,
            }
        }
    }

    #[inline]
    pub fn rational(r: Ratio<u32>) -> Self {
        Self::signed_rational(Sign::Plus, r)
    }

    #[inline]
    pub const fn flip_sign(&mut self) {
        self.sign = self.sign.flip();
    }

    /// `0` is neither negative nor positive
    #[inline]
    pub const fn is_negative(self) -> bool {
        !self.is_zero() && self.sign.is_minus()
    }

    /// `0` is neither negative nor positive
    #[inline]
    pub const fn is_positive(self) -> bool {
        !self.is_zero() && self.sign.is_plus()
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        matches!(self.typ, RealTyp::Zero)
    }

    #[inline]
    pub fn abs(self) -> Self {
        Self {
            typ: self.typ,
            sign: Sign::Plus,
        }
    }

    #[inline]
    pub fn sign(&self) -> Option<Sign> {
        match self.typ {
            RealTyp::Zero => None,
            _ => Some(self.sign),
        }
    }

    /// will calculate the simplified form of `self` to the power of another [`Real`]
    ///
    /// if the exponent is an integer the function returns `(base^expon, None)`
    ///
    /// if the exponent is (a/b) non-int: we calculate the power to the int quotient of a/b
    /// and return the remainder: (self^quot, rem).
    ///
    /// `base^(a/b) = base^(quot + rem) -> (base^quot, rem)`
    ///
    #[inline]
    pub fn pow_simplify(self, exp: Real) -> (Self, Option<Self>) {
        let (s1, r1) = self.as_rational();
        let (s2, r2) = exp.as_rational();
        pow_ratio(s1, r1, s2, r2)
    }

    #[inline]
    pub fn pow(self, exp: Real) -> Self {
        match (self.typ, exp.typ) {
            (RealTyp::Zero, RealTyp::Zero) => panic!("0^0"),
            (RealTyp::Zero, RealTyp::U32(_)) => {
                if exp.sign.is_minus() {
                    panic!("div by 0")
                } else {
                    self
                }
            }
            (_, RealTyp::Zero) => Real::u32(1),
            (RealTyp::U32(b), RealTyp::U32(e)) if exp.is_negative() => {
                let mut sb = self.sign;
                if e % 2 == 0 {
                    sb = Sign::Plus;
                }
                Real::signed_rational(sb, Ratio::new(1, b))
            }
            (RealTyp::U32(b), RealTyp::U32(e)) => {
                let mut sb = self.sign;
                if e % 2 == 0 {
                    sb = Sign::Plus;
                }
                Real::signed_u32(sb, b.pow(e))
            }
            (_, RealTyp::Ratio(r)) | (RealTyp::Ratio(r), _) => {
                let (l, r) = (self.as_rational(), exp.as_rational());
                todo!()
            }
        }
    }

    #[inline]
    fn as_rational(self) -> (Sign, Ratio<u32>) {
        let r = match self.typ {
            RealTyp::Zero => Ratio::ZERO,
            RealTyp::U32(u) => Ratio::new(u, 1),
            RealTyp::Ratio(r) => r,
        };

        (self.sign, r)
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.typ {
            RealTyp::Zero => write!(f, "0"),
            RealTyp::U32(u) => write!(f, "{}{u}", self.sign.fmt_prefix()),
            RealTyp::Ratio(r) => write!(f, "{}{r}", self.sign.fmt_prefix()),
        }
    }
}

impl ops::Neg for Real {
    type Output = Real;

    fn neg(mut self) -> Self::Output {
        self.flip_sign();
        self
    }
}

impl ops::Add for Real {
    type Output = Real;

    fn add(self, rhs: Self) -> Self::Output {
        let (s1, l) = self.as_rational();
        let (s2, r) = rhs.as_rational();

        match (s1, s2) {
            (Sign::Plus, Sign::Plus) => Real::signed_rational(Sign::Plus, l + r),
            (Sign::Minus, Sign::Minus) => Real::signed_rational(Sign::Minus, l + r),
            (Sign::Minus, Sign::Plus) => todo!(),
            (Sign::Plus, Sign::Minus) => {
                if l >= r {
                    Real::signed_rational(Sign::Plus, l - r)
                } else {
                    Real::signed_rational(Sign::Minus, r - l)
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
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl ops::Mul for Real {
    type Output = Real;

    fn mul(self, rhs: Self) -> Self::Output {
        let (s1, l) = self.as_rational();
        let (s2, r) = rhs.as_rational();
        Real::signed_rational(s1*s2, l*r)
    }
}

impl ops::MulAssign for Real {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl ops::Div for Real {
    type Output = Real;

    fn div(self, rhs: Self) -> Self::Output {
        let (s1, l) = self.as_rational();
        let (s2, r) = rhs.as_rational();
        Real::signed_rational(s1*s2, l/r)
    }
}
impl ops::DivAssign for Real {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
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
