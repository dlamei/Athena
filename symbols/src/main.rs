use std::{borrow::Cow, cell::{Cell, RefCell, UnsafeCell}, collections::VecDeque, fmt, ops, rc::Rc};

use itertools::Itertools;

/// A double-ended queue (`VecDeque`) wrapped to provide linear (contiguous) slices on demand.
///
/// # Overview
/// `FlatDeque<T>` holds a `VecDeque<T>` inside an `UnsafeCell` to allow converting a possibly-wrapped deque into
/// a contiguous slice 
///
/// # Examples
///
/// ```rust
/// let mut dq = FlatDeque::new();
/// dq.push_back(1);
/// dq.push_front(0);
/// assert_eq!(dq.as_slice(), &[0, 1]);
/// dq.pop_front();
/// assert_eq!(dq.as_mut_slice(), &mut [1]);
/// ```
pub struct FlatDeque<T> {
    deque: UnsafeCell<VecDeque<T>>,
}

// we can't just implement Deref because [`as_slice`] would be UB after calling [`std::collections::VecDeque::as_slices`]
impl<T> FlatDeque<T> {

    #[inline]
    pub const fn new() -> Self {
        Self {
            deque: UnsafeCell::new(VecDeque::new()),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            deque: UnsafeCell::new(VecDeque::with_capacity(capacity)) 
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        unsafe { &*self.deque.get() }.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.deque.get_mut().get_mut(index)
    }

    #[inline]
    pub fn insert(&mut self, index: usize, value: T) {
        self.deque.get_mut().insert(index, value)
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.deque.get_mut().remove(index)
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.deque.get_mut().swap(i, j)
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.deque.get_mut().reserve(additional)
    }

    #[inline]
    pub fn push_front(&mut self, value: T) {
        self.deque.get_mut().push_front(value);
    }

    #[inline]
    pub fn push_back(&mut self, value: T) {
        self.deque.get_mut().push_back(value)
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        self.deque.get_mut().pop_front()
    }

    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        self.deque.get_mut().pop_back()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.deque.get_mut().clear()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { &*self.deque.get() }.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { &*self.deque.get() }.capacity()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_mut_slice().iter_mut()
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { 
        self.deque.get_mut().extend(iter)
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.deque.get_mut().append(other.deque.get_mut())
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.deque.get_mut().retain(f)
    }

    #[inline]
    pub fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        self.deque.get_mut().retain_mut(f)
    }

    #[inline]
    pub fn make_contiguous(&mut self) -> &mut [T] {
        self.deque.get_mut().make_contiguous()
    }

    #[inline]
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> std::cmp::Ordering,
    {
        unsafe { &*self.deque.get() }.binary_search_by(f)
    }

    #[inline]
    pub fn binary_search(&self, x: &T) -> Result<usize, usize>
    where
        T: Ord,
    {
        self.binary_search_by(|e| e.cmp(x))
    }

    #[inline]
    pub fn binary_search_by_key<'a, B, F>(&'a self, b: &B, mut f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> B,
        B: Ord,
    {
        self.binary_search_by(|k| f(k).cmp(b))
    }

    #[inline]
    pub fn partition_point<P>(&self, pred: P) -> usize
    where
        P: FnMut(&T) -> bool,
    {
        unsafe { &*self.deque.get() }.partition_point(pred)
    }

    /// Returns a contiguous slice of all elements.
    ///
    /// This may re-align the internal buffer if the data wraps around.
    ///
    /// # Safety
    ///
    /// - We temporarily cast a `&self` to a `&mut VecDeque<T>` only when the data
    ///   is not already linear. For subsequent calls to this function we guarantee
    ///   the underlying data is never modified as long as other slices still exist.
    ///
    /// - The returned slice's lifetime is bound to `&self`, preventing any
    ///   mutation (including reallocation) while the slice is alive.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let dq = unsafe { &*self.deque.get() };
        let (s1, s2) = dq.as_slices();
        if s2.is_empty() {
            return s1;
        }

        let dq_mut = unsafe { &mut *self.deque.get() };
        dq_mut.make_contiguous()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.deque.get_mut().make_contiguous()
    }
}

impl<T> Default for FlatDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl <T: Clone> Clone for FlatDeque<T> {
    fn clone(&self) -> Self {
        let deque = unsafe { &*self.deque.get() }.clone();
        Self { deque: UnsafeCell::new(deque) }
    }
}

impl <T: fmt::Debug> fmt::Debug for FlatDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice().iter()).finish()
    }
}

impl <T: PartialEq> PartialEq for FlatDeque<T> {
    fn eq(&self, other: &Self) -> bool {
        let dq1 = unsafe { &*self.deque.get() };
        let dq2 = unsafe { &*other.deque.get() };
        dq1.eq(dq2)
    }
}

impl <T: Eq> Eq for FlatDeque<T> {}

impl <T: PartialOrd> PartialOrd for FlatDeque<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let dq1 = unsafe { &*self.deque.get() };
        let dq2 = unsafe { &*other.deque.get() };
        dq1.partial_cmp(dq2)
    }
}

impl<T: Ord> Ord for FlatDeque<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let dq1 = unsafe { &*self.deque.get() };
        let dq2 = unsafe { &*other.deque.get() };
        dq1.cmp(dq2)
    }
}

impl<T: std::hash::Hash> std::hash::Hash for FlatDeque<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { &*self.deque.get() }.hash(state)
    }
}


impl<T> From<Vec<T>> for FlatDeque<T> {
    fn from(value: Vec<T>) -> Self {
        let mut dq = VecDeque::from(value);
        dq.make_contiguous();
        FlatDeque { deque: UnsafeCell::new(dq) }
    }
}

impl<T, const N: usize> From<[T; N]> for FlatDeque<T> {
    fn from(value: [T; N]) -> Self {
        value.into_iter().collect()
    }
}

impl<T> FromIterator<T> for FlatDeque<T> {
    fn from_iter<S: IntoIterator<Item = T>>(iter: S) -> Self {
        let vec: Vec<T> = iter.into_iter().collect();
        Self::from(vec)
    }
}

impl<T> IntoIterator for FlatDeque<T> {
    type Item = T;
    type IntoIter = std::collections::vec_deque::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deque.into_inner().into_iter()
    }
}

impl<T> ops::Index<usize> for FlatDeque<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &(unsafe { &*self.deque.get() })[index]
    }
}

impl<T> ops::IndexMut<usize> for FlatDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut (unsafe { &mut *self.deque.get() })[index]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn linear_deque() {
        let mut dq = FlatDeque::new();
        let mut dq_ref = VecDeque::new();

        for i in 0..1000_000 {
            if i % 2 == 0 {
                dq.push_front(i);
                dq_ref.push_front(i);
            } else {
                dq.push_back(i);
                dq_ref.push_back(i);
            }
            
            if i % 100 == 0 {
                let _ = dq.as_slice();
            }
        }

        dq.into_iter().zip(dq_ref.make_contiguous().iter()).for_each(|(a, &b)| {
            assert_eq!(a, b);
        });

    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtomRef<'a> {
    Undef,
    U32(u32),
    Var(&'a str),
    Neg(&'a Atom),
    Sum(&'a [Atom]),
    Prod(&'a [Atom]),
    Sub(&'a [Atom; 2]),
    Div(&'a [Atom; 2]),
    Pow(&'a [Atom; 2]),
}

impl<'a> AtomRef<'a> {
    pub fn is_i32(&self, val: i32) -> bool {
        match self {
            AtomRef::U32(v) if val >= 0 => *v == val.unsigned_abs(),
            AtomRef::Neg(Atom::U32(v)) if val < 0 => *v == val.unsigned_abs(),
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AtomMut<'a> {
    Undef(&'a mut Atom),
    U32(&'a mut u32),
    Atom(&'a mut Atom),
    Neg(&'a mut Atom),
    Sum(&'a mut [Atom]),
    Prod(&'a mut [Atom]),
    Sub(&'a mut [Atom; 2]),
    Div(&'a mut [Atom; 2]),
    Pow(&'a mut [Atom; 2]),
}

impl<'a> AtomMut<'a> {
    pub fn atom_ref(&'a self) -> AtomRef<'a> {
        match self {
            AtomMut::Undef(_) => AtomRef::Undef,
            AtomMut::U32(val) => AtomRef::U32(**val),
            AtomMut::Atom(atom) | AtomMut::Neg(atom) => atom.atom_ref(),
            AtomMut::Sum(oprnds) => AtomRef::Sum(&**oprnds),
            AtomMut::Prod(oprnds) => AtomRef::Prod(&**oprnds),
            AtomMut::Sub(oprnds) => AtomRef::Sub(&**oprnds),
            AtomMut::Div(oprnds) => AtomRef::Div(&**oprnds),
            AtomMut::Pow(oprnds) => AtomRef::Pow(&**oprnds),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Undef,
    U32(u32),
    Var(Rc<str>),
    Expr(Rc<Expr>),
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Undef => write!(f, "\u{2205}"),
            Atom::U32(val) => write!(f, "{val}"),
            Atom::Var(var) => write!(f, "{}", *var),
            Atom::Expr(expr) => write!(f, "{expr}"),
        }
    }
}

impl Atom {
    pub fn atom_ref<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Atom::Undef => AtomRef::Undef,
            Atom::U32(val) => AtomRef::U32(*val),
            Atom::Var(var) => AtomRef::Var(var.as_ref()),
            Atom::Expr(expr) => expr.atom_ref(),
        }
    }

    pub fn atom_mut<'a>(&'a mut self) -> AtomMut<'a> {
        match self {
            Atom::Undef
            | Atom::U32(_)
            | Atom::Var(_) => AtomMut::Atom(self),
            Atom::Expr(expr) => Rc::make_mut(expr).atom_mut(),
        }
    }

}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Atom(Atom),
    Neg(Atom),

    Sum(FlatDeque<Atom>),
    Prod(FlatDeque<Atom>),

    Sub([Atom; 2]),
    Div([Atom; 2]),
    Pow([Atom; 2]),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Atom(atom) => write!(f, "{atom}"),
            Expr::Neg(Atom::Expr(expr)) => write!(f, "-({expr})"),
            Expr::Neg(atom) => write!(f, "-{atom}"),
            Expr::Sum(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]")
                } 
                else if oprnds.len() == 1 {
                    return write!(f, "[+{}]", oprnds[0])
                }
                write!(f, "{}", oprnds.iter().format(" + "))
            },
            Expr::Prod(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]")
                } else if oprnds.len() == 1 {
                    return write!(f, "[*{}]", oprnds[0])
                }
                write!(f, "{}", oprnds.iter().format(" * "))
            }
            Expr::Sub([lhs, rhs]) => {
                write!(f, "{lhs} - ")?;
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
            Expr::Div([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_))  {
                    write!(f, "({lhs})/")?;
                } else {
                    write!(f, "{lhs}/")?;
                }
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
            Expr::Pow([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_))  {
                    write!(f, "({lhs})^")?;
                } else {
                    write!(f, "{lhs}^")?;
                }
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
        }
    }
}

impl Expr {

    pub fn as_atom(self) -> Atom {
        match self {
            Self::Atom(atom) => atom,
            _ => Atom::Expr(self.into()),
        }
    }

    pub fn atom_ref<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(atom) => atom.atom_ref(),
            Expr::Neg(atom) => AtomRef::Neg(atom),
            Expr::Sum(oprnds) => AtomRef::Sum(oprnds.as_slice()),
            Expr::Prod(oprnds) => AtomRef::Prod(oprnds.as_slice()),
            Expr::Sub(oprnds) => AtomRef::Sub(oprnds),
            Expr::Div(oprnds) => AtomRef::Div(oprnds),
            Expr::Pow(oprnds) => AtomRef::Pow(oprnds),
        }
    }

    pub fn atom_mut<'a>(&'a mut self) -> AtomMut<'a> {
        match self {
            Expr::Atom(atom) => atom.atom_mut(),
            Expr::Neg(atom) => AtomMut::Neg(atom),
            Expr::Sum(oprnds) => AtomMut::Sum(oprnds.as_mut_slice()),
            Expr::Prod(oprnds) => AtomMut::Prod(oprnds.as_mut_slice()),
            Expr::Sub(oprnds) => AtomMut::Sub(oprnds),
            Expr::Div(oprnds) => AtomMut::Div(oprnds),
            Expr::Pow(oprnds) => AtomMut::Pow(oprnds),
        }
    }

    pub fn oprnds<'a>(&'a self) -> &[Atom] {
        match self {
            Expr::Atom(atom) | Expr::Neg(atom) => std::slice::from_ref(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    pub fn oprnds_mut(&mut self) -> &mut [Atom] {
        match self {
            Expr::Atom(atom) | Expr::Neg(atom) => std::slice::from_mut(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_mut_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }


    pub fn base<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(_)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => self.atom_ref(),
            Expr::Pow([base, _]) => base.atom_ref(),
        }
    }

    pub fn exponent<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(_) | Expr::Neg(_) | Expr::Sum(_) | Expr::Prod(_) | Expr::Sub(_) | Expr::Div(_) => AtomRef::U32(1),
            Expr::Pow([_, expon]) => expon.atom_ref(),
        }
    }

    pub fn collect_numer_denom(&self) -> Self {
        let numer = Expr::from(1);
        let denom = Expr::from(1);
        todo!()
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Expr::Atom(Atom::U32(value))
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        let mut atom = Atom::U32(value.unsigned_abs());
        if value > 0 {
            Expr::Atom(atom)
        } else {
            Expr::Neg(atom)
        }
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Self::Atom(Atom::Var(value.into()))
    }
}

impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Sub([self.as_atom(), rhs.as_atom()].into())
    }
}

impl ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Sum(mut p1), Self::Sum(p2)) => {
                p1.extend(p2);
                Self::Sum(p1)
            }
            (Self::Sum(mut p), rhs) => {
                p.push_back(rhs.as_atom());
                Self::Sum(p)
            }
            (lhs, Self::Sum(mut p)) => {
                p.push_front(lhs.as_atom());
                Self::Sum(p)
            }
            (lhs, rhs) => Self::Sum([lhs.as_atom(), rhs.as_atom()].into()),
        }
    }
}

impl ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Prod(mut p1), Self::Prod(p2)) => {
                p1.extend(p2);
                Self::Prod(p1)
            }
            (Self::Prod(mut p), rhs) => {
                p.push_back(rhs.as_atom());
                Self::Prod(p)
            }
            (lhs, Self::Prod(mut p)) => {
                p.push_front(lhs.as_atom());
                Self::Prod(p)
            }
            (lhs, rhs) => Self::Prod([lhs.as_atom(), rhs.as_atom()].into()),
        }
    }
}

impl ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Self) -> Self::Output {
        Self::Div([self.as_atom(), rhs.as_atom()])
    }
}

fn main() {

    let val = Expr::from(-34);
    let add = Expr::from(34) + val.clone();
    let mut a = Vec::new();
    a.push(3);
    println!("{}", add);
    println!("{}", Expr::Atom(Atom::Undef));

}
