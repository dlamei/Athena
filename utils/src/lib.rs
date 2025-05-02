use std::fmt;

pub trait ExplicitCopy: Copy {
    #[inline(always)]
    fn copy(&self) -> Self {
        *self
    }
}

impl<T: Copy> ExplicitCopy for T {}

type WRD = u64;
const N_WRD_BITS: u32 = 64;

pub struct BitGrid {
    pub data: Box<[u64]>,
    pub width: u32,
    pub height: u32,
    pub count: u32,
}

impl fmt::Display for BitGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let ch = if self.get(x, y) { 'x' } else { 'o' };
                write!(f, "{}", ch)?;
            }
            if y < self.height - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl BitGrid {
    #[inline]
    pub fn new(w: u32, h: u32) -> Self {
        let count = w * h;
        let n_words = (count + N_WRD_BITS - 1) >> 6;
        let data = vec![0; n_words as usize].into_boxed_slice();
        Self {
            data,
            width: w,
            height: h,
            count,
        }
    }

    #[inline]
    pub fn idx_from_xy(&self, x: u32, y: u32) -> (usize, u8) {
        let bit_off = y * self.width + x;
        (
            (bit_off / N_WRD_BITS) as usize,
            (bit_off % N_WRD_BITS) as u8,
        )
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32) {
        debug_assert!(
            x < self.width && y < self.height,
            "x: {x}, y: {y}, w: {}, h: {}",
            self.width,
            self.height
        );
        let bit = y * self.width + x;
        let word = (bit >> 6) as usize;
        let offset = (bit & (N_WRD_BITS - 1)) as u64;

        self.data[word] |= 1 << offset;
        // debug_assert!(word < self.data.len());
        // unsafe {
        //     *self.data.get_unchecked_mut(word) |= 1 << offset;
        // }
    }

    // pub fn set(&mut self, x: u32, y: u32) {
    //     let (word_off, bit_off) = self.idx_from_xy(x, y);
    //     self.data[word_off] |= 1 << bit_off;
    // }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> bool {
        debug_assert!(
            x < self.width && y < self.height,
            "x: {x}, y: {y}, w: {}, h: {}",
            self.width,
            self.height
        );
        let (word_off, bit_off) = self.idx_from_xy(x, y);
        self.data[word_off] & (1 << bit_off) != 0
    }

    #[inline]
    pub fn iter<'a>(&'a self) -> BitIter<'a> {
        if self.count == 0 {
            BitIter::default()
        } else {
            let last_wrd_i = (self.count - 1) / N_WRD_BITS;
            let valid_bits_last_wrd = self.count - (last_wrd_i * N_WRD_BITS);
            let last_wrd_mask = if valid_bits_last_wrd == N_WRD_BITS {
                WRD::MAX
            } else {
                (1 << valid_bits_last_wrd) - 1
            };

            let curr_wrd = self.data.first().copied().unwrap_or(0)
                & if 0 == last_wrd_i {
                    last_wrd_mask
                } else {
                    u64::MAX
                };

            BitIter {
                data: &self.data,
                width: self.width,
                index: 0,
                curr_wrd,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitIter<'a> {
    pub data: &'a [WRD],
    pub width: u32,
    pub index: usize,
    pub curr_wrd: WRD,
}

impl Default for BitIter<'_> {
    fn default() -> Self {
        Self {
            data: &[],
            width: 0,
            index: 0,
            curr_wrd: 0,
        }
    }
}

impl Iterator for BitIter<'_> {
    type Item = (u32, u32);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.curr_wrd == 0 {
            self.index += 1;
            let idx = self.index;
            if idx >= self.data.len() {
                return None;
            }
            self.curr_wrd = self.data[idx];
        }

        let tz = self.curr_wrd.trailing_zeros();
        let bit_indx = self.index as u32 * N_WRD_BITS + tz;
        let x = bit_indx % self.width;
        let y = bit_indx / self.width;
        self.curr_wrd &= self.curr_wrd - 1;

        Some((x, y))
    }
}

#[macro_export]
macro_rules! cnst_grid_ty {
    ($w:expr, $h:expr) => {
        $crate::CnstBitGrid::<{ $w }, { $h }, { ((($w * $h) + 64 - 1) >> 6) }>
    };
}

#[macro_export]
macro_rules! cnst_grid {
    ($w:expr, $h:expr) => {
        <$crate::cnst_grid_ty!($w, $h)>::new()
    };
}

pub struct CnstBitGrid<const W: u32, const H: u32, const N_WRDS: usize> {
    data: [u64; N_WRDS],
}

impl<const W: u32, const H: u32, const N_WRDS: usize> CnstBitGrid<W, H, N_WRDS> {
    #[inline]
    pub fn new() -> Self {
        assert!(N_WRDS as u32 * 64 >= W * H);
        Self { data: [0; N_WRDS] }
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32) {
        debug_assert!(x < W && y < H, "x: {x}, y: {y}, w: {}, h: {}", W, H,);
        let bit = y * W + x;
        let word = (bit >> 6) as usize;
        let offset = (bit & (N_WRD_BITS - 1)) as u64;

        debug_assert!(word < self.data.len());
        unsafe {
            *self.data.get_unchecked_mut(word) |= 1 << offset;
        }
    }

    pub fn idx_from_xy(&self, x: u32, y: u32) -> (usize, u8) {
        let bit_off = y * W + x;
        ((bit_off / 64) as usize, (bit_off % 64) as u8)
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        debug_assert!(x < W && y < H, "x: {x}, y: {y}, w: {}, h: {}", W, H,);
        let (word_off, bit_off) = self.idx_from_xy(x, y);
        self.data[word_off] & (1 << bit_off) != 0
    }

    pub fn iter<'a>(&'a self) -> CnstBitIter<'a, W, H, N_WRDS> {
        let count = W * H;
        let last_wrd_i = (count - 1) / 64;
        let valid_bits_last_wrd = count - (last_wrd_i * 64);
        let last_wrd_mask = if valid_bits_last_wrd == 64 {
            WRD::MAX
        } else {
            (1 << valid_bits_last_wrd) - 1
        };

        let curr_wrd = self.data.first().copied().unwrap_or(0)
            & if 0 == last_wrd_i {
                last_wrd_mask
            } else {
                u64::MAX
            };

        CnstBitIter {
            data: &self.data,
            last_wrd_i,
            last_wrd_mask,
            curr_wrd_i: 0,
            curr_wrd,
        }
    }
}

pub struct CnstBitIter<'a, const W: u32, const H: u32, const N: usize> {
    pub data: &'a [u64; N],
    pub last_wrd_i: u32,
    pub last_wrd_mask: WRD,
    pub curr_wrd_i: u32,
    pub curr_wrd: WRD,
}

impl<const W: u32, const H: u32, const N: usize> Iterator for CnstBitIter<'_, W, H, N> {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        while self.curr_wrd == 0 {
            self.curr_wrd_i += 1;
            let idx = self.curr_wrd_i as usize;
            if idx >= self.data.len() {
                return None;
            }

            self.curr_wrd = self.data[idx];

            if self.curr_wrd_i == self.last_wrd_i {
                self.curr_wrd &= self.last_wrd_mask;
            }
        }

        let bit_pos = self.curr_wrd.trailing_zeros();
        let glob_offst = self.curr_wrd_i * N_WRD_BITS + bit_pos;
        let x = glob_offst % W;
        let y = glob_offst / W;
        self.curr_wrd &= self.curr_wrd - 1;

        Some((x, y))
    }
}

macro_rules! min_max {
    ($a:expr, $b:expr $(,)?) => {
        if $a < $b { ($a, $b) } else { ($b, $a) }
    };

    ($a:expr, $b:expr, $c:expr, $d:expr $(,)?) => {
        (min_max!($a, $b).0, min_max!($c, $d).1)
    };
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Intrvl {
    pub lo: f64,
    pub hi: f64,
}

impl From<(f64, f64)> for Intrvl {
    fn from((lo, hi): (f64, f64)) -> Self {
        Self::new(lo, hi)
    }
}

impl fmt::Display for Intrvl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lo, self.hi)
    }
}

impl Intrvl {
    pub const UNDEF: Self = Self {
        lo: f64::NAN,
        hi: f64::NAN,
    };
    pub const WHOLE: Self = Self {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    #[inline]
    pub const fn undef() -> Self {
        Self::UNDEF
    }

    #[inline]
    pub const fn whole() -> Self {
        Self::WHOLE
    }

    #[inline]
    pub const fn new(lo: f64, hi: f64) -> Self {
        if lo.is_nan() || hi.is_nan() || lo > hi {
            return Self::undef();
        }

        Self { lo, hi }
    }

    #[inline]
    pub const fn scalar(v: f64) -> Self {
        Self { lo: v, hi: v }
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.lo > self.hi || self.lo.is_nan() || self.hi.is_nan()
    }

    #[inline]
    pub const fn is_scalar(&self) -> bool {
        (self.hi - self.lo).abs() <= f64::EPSILON
    }

    #[inline]
    pub fn is_scalar_int(&self) -> bool {
        self.is_scalar() && self.lo.fract().abs() <= f64::EPSILON
    }

    #[inline]
    pub const fn contains_zero(&self) -> bool {
        self.lo <= f64::EPSILON && self.hi >= -f64::EPSILON && !self.is_valid()
    }

    #[inline]
    pub const fn intersect(self, o: Self) -> Self {
        let (lo, hi) = min_max!(self.lo, self.hi, o.lo, o.hi);
        Self::new(lo, hi)
    }

    /// x + y = [x_min + y_min, x_max + y_max]
    #[inline]
    pub const fn add(self, o: Self) -> Self {
        let lo = self.lo + o.lo;
        let hi = self.hi + o.hi;
        Self { lo, hi }
    }

    /// x - y = [x_min - y_max, x_max - y_min]
    #[inline]
    pub const fn sub(self, o: Self) -> Self {
        let lo = self.lo - o.hi;
        let hi = self.hi - o.lo;
        Self { lo, hi }
    }

    /// x * y = min_max(x_min*y_min, x_min*y_max, x_max*y_min, x_max*y_max)
    #[inline]
    pub const fn mul(self, o: Self) -> Self {
        let (lo, hi) = min_max![
            self.lo * o.lo,
            self.lo * o.hi,
            self.hi * o.lo,
            self.hi * o.hi,
        ];
        Self { lo, hi }
    }

    /// x / y = x * [1/y_max, 1/y_min]
    #[inline]
    pub const fn div(self, o: Self) -> Self {
        if self.contains_zero() {
            return Self::undef();
        }

        let inv_o = Self {
            lo: 1.0 / o.hi,
            hi: 1.0 / o.lo,
        };
        self.mul(inv_o)
    }

    #[inline]
    pub fn exp(self) -> Self {
        let lo = self.lo.exp();
        let hi = self.hi.exp();
        Self { lo, hi }
    }

    #[inline]
    pub fn ln(self) -> Self {
        if self.hi <= f64::EPSILON {
            return Self::undef();
        }
        let lo = if self.lo > 0.0 {
            self.lo.ln()
        } else {
            f64::NEG_INFINITY
        };
        let hi = self.hi.ln();
        Self { lo, hi }
    }

    #[inline]
    pub fn powf(self, n: f64) -> Self {
        if self.lo <= f64::EPSILON {
            return Self::undef();
        }
        // TODO: handle integer exponents?

        let (lo, hi) = min_max![self.lo.powf(n), self.hi.powf(n),];
        Self { lo, hi }
    }

    /// x^y = exp(y * ln(x))
    #[inline]
    pub fn pow(self, e: Self) -> Self {
        self.ln().mul(e).exp()
    }

    #[inline]
    pub fn sin(self) -> Self {
        use std::f64::consts;
        const TWO_PI: f64 = 2.0 * consts::PI;
        const HALF_PI: f64 = consts::FRAC_PI_2;
        const THREE_HALF_PI: f64 = 3.0 * HALF_PI;

        if self.hi - self.lo >= TWO_PI {
            return Self { lo: -1.0, hi: 1.0 };
        }

        let lo = self.lo.rem_euclid(TWO_PI);
        let hi = self.hi.rem_euclid(TWO_PI);

        let contains_half_pi =
            (lo <= HALF_PI && HALF_PI <= hi) || (lo > hi && (HALF_PI >= lo || HALF_PI <= hi));
        let contains_three_halves_pi = (lo <= THREE_HALF_PI && THREE_HALF_PI <= hi)
            || (lo > hi && (THREE_HALF_PI >= lo || THREE_HALF_PI <= hi));

        let (raw_lo, raw_hi) = if contains_three_halves_pi && contains_half_pi {
            (-1.0, 1.0)
        } else if contains_three_halves_pi && !contains_half_pi {
            (-1.0, lo.sin().max(hi.sin()))
        } else if !contains_three_halves_pi && contains_half_pi {
            (lo.sin().min(hi.sin()), 1.0)
        } else {
            min_max!(lo.sin(), hi.sin())
        };

        Self {
            lo: raw_lo,
            hi: raw_hi,
        }
    }

    // #[inline]
    // pub fn sin(self) -> Self {
    //     use std::f64::consts;
    //     const TWO_PI: f64 = 2.0 * consts::PI;
    //     const HALF_PI: f64 = consts::FRAC_PI_2;
    //     const THREE_HALF_PI: f64 = 3.0 * HALF_PI;

    //     let mut a = self.lo % TWO_PI;
    //     let mut b = self.hi % TWO_PI;
    //     if a < 0.0 { a += TWO_PI }
    //     if b < 0.0 { b += TWO_PI }
    //     if b - a >= TWO_PI {
    //         return Self { lo: -1.0, hi: 1.0 };
    //     }
    //     let fa = a.sin();
    //     let fb = b.sin();
    //     let (mut raw_lo, mut raw_hi) = min_max!(fa, fb);

    //     if a <= HALF_PI && HALF_PI <= b { raw_hi = 1.0; }
    //     if a <= THREE_HALF_PI && THREE_HALF_PI <= b { raw_lo = -1.0; }

    //     let lo = f64_next(raw_lo).down();
    //     let hi = f64_next(raw_hi).up();
    //     Self { lo, hi }
    // }

    #[inline]
    pub fn cos(self) -> Self {
        const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
        Self {
            lo: self.lo + HALF_PI,
            hi: self.hi + HALF_PI,
        }
        .sin()
        // const PI: f64 = std::f64::consts::PI;
        // const TWO_PI: f64 = 2.0 * PI;

        // if self.is_empty() {
        //     return Self::empty()
        // } else if self.hi - self.lo >= TWO_PI {
        //     return Self { lo: -1.0, hi: 1.0 }
        // }

        // let lo = self.lo.rem_euclid(TWO_PI);
        // let hi = self.hi.rem_euclid(TWO_PI);

        // let contains_zero = (lo <= 0.0 && 0.0 <= hi) || (lo > hi && (0.0 >= lo || 0.0 <= hi));
        // let contains_pi = (lo <= PI && PI <= hi) || (lo > hi && (PI >= lo || PI <= hi));

        // let min = if contains_pi {
        //     -1.0
        // } else {
        //     lo.cos().min(hi.cos())
        // };
        // let max = if contains_zero {
        //     1.0
        // } else {
        //     lo.cos().max(hi.cos())
        // };

        // let (raw_lo, raw_hi) = if contains_pi && contains_zero {
        //     (-1.0, 1.0)
        // } else if contains_pi && !contains_zero {
        //     (-1.0, lo.cos().max(hi.cos()))
        // } else if !contains_pi && contains_zero {
        //     (lo.cos().min(hi.cos()), 1.0)
        // } else {
        //     min_max!(lo.cos(), hi.cos())
        // };

        // Self { lo: f64_next(raw_lo).down(), hi: f64_next(raw_hi).up() }
    }

    #[inline]
    pub fn tan(self) -> Self {
        const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
        const PI: f64 = std::f64::consts::PI;

        let k_lo = ((self.lo + HALF_PI) / PI).floor();
        let k_hi = ((self.hi + HALF_PI) / PI).floor();
        if (k_hi - k_lo) >= 1.0 {
            return Self::undef();
        }

        let lo = self.lo.tan();
        let hi = self.hi.tan();
        Self { lo, hi }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn packed_const_grid() {
        let mut g = cnst_grid!(4, 4);
        g.set(1, 1);
        g.set(3, 1);
        g.set(3, 3);

        assert!(g.get(1, 1));
        assert!(g.get(3, 1));
        assert!(g.get(3, 3));

        let bits: Vec<_> = g.iter().collect();
        assert_eq!(&bits, &[(1, 1), (3, 1), (3, 3)], "{bits:?}");
    }

    #[test]
    fn packed_grid() {
        let mut g = BitGrid::new(4, 4);
        g.set(1, 1);
        g.set(3, 1);
        g.set(3, 3);

        assert!(g.get(1, 1));
        assert!(g.get(3, 1));
        assert!(g.get(3, 3));

        let bits: Vec<_> = g.iter().collect();
        assert_eq!(&bits, &[(1, 1), (3, 1), (3, 3)], "{bits:?}");
    }
}
