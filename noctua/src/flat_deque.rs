use std::{cell::UnsafeCell, collections::{vec_deque, VecDeque}, fmt, ops};

/// A draining iterator over the elements of a `FlatDeque`.
///
/// This `struct` is created by the [`drain`] method on [`FlatDeque`]. 
///
/// [`drain`]: VecDeque::drain
pub type Drain<'a, T> = vec_deque::Drain<'a, T>;

/// A double-ended queue (`VecDeque`) wrapped to provide linear (contiguous) slices on demand.
///
/// # Overview
/// `FlatDeque<T>` holds a `VecDeque<T>` inside an `UnsafeCell` to allow converting a possibly-wrapped deque into
/// a contiguous slice
///
/// # Examples
///
/// ```rust
/// # use noctua::flat_deque::FlatDeque;
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
            deque: UnsafeCell::new(VecDeque::with_capacity(capacity)),
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
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
    pub fn front(&self) -> Option<&T> {
        self.as_slice().first()
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.as_slice().last()
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
        // SAFETY: no mutable reference can exist simultaneously and the result is unaffected by
        // make_contiguous
        unsafe { &*self.deque.get() }.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        // SAFETY: no mutable reference can exist simultaneously and the result is unaffected by
        // make_contiguous
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
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T> 
    where
        R: ops::RangeBounds<usize>,
    {
        self.deque.get_mut().drain(range)
    }

    #[inline]
    pub fn make_contiguous(&mut self) {
        self.deque.get_mut().make_contiguous();
    }

    #[inline]
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> std::cmp::Ordering,
    {
        // SAFETY: no mutable reference can exist simultaneously and the result is unaffected by
        // make_contiguous
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
        // SAFETY: no mutable reference can exist simultaneously and the result is unaffected by
        // make_contiguous
        unsafe { &*self.deque.get() }.partition_point(pred)
    }

    /// Returns a contiguous slice of all elements.
    ///
    /// This may re-align the internal buffer if the data wraps around.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY:
        //
        // - We temporarily cast a `&self` to a `&mut VecDeque<T>` only when the data
        //   is not already linear. For subsequent calls to this function we guarantee
        //   the underlying data is never modified as long as other slices still exist.
        //
        // - The returned slice's lifetime is bound to `&self`, preventing any
        //   mutation (including reallocation) while the slice is alive.

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

impl<T: Clone> Clone for FlatDeque<T> {
    fn clone(&self) -> Self {
        let deque = unsafe { &*self.deque.get() }.clone();
        Self {
            deque: UnsafeCell::new(deque),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FlatDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice().iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for FlatDeque<T> {
    fn eq(&self, other: &Self) -> bool {
        let dq1 = unsafe { &*self.deque.get() };
        let dq2 = unsafe { &*other.deque.get() };
        dq1.eq(dq2)
    }
}

impl<T: Eq> Eq for FlatDeque<T> {}

impl<T: PartialOrd> PartialOrd for FlatDeque<T> {
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
        FlatDeque {
            deque: UnsafeCell::new(dq),
        }
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
    type IntoIter = vec_deque::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deque.into_inner().into_iter()
    }
}

impl<T> ops::Index<usize> for FlatDeque<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T> ops::Index<ops::Range<usize>> for FlatDeque<T> {
    type Output = [T];
    fn index(&self, index: ops::Range<usize>) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T> ops::IndexMut<usize> for FlatDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<T> ops::IndexMut<ops::Range<usize>> for FlatDeque<T> {
    fn index_mut(&mut self, index: ops::Range<usize>) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn linear_deque() {
        let mut dq = FlatDeque::new();
        let mut dq_ref = VecDeque::new();

        for i in 0..1_000_000 {
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

        dq.into_iter()
            .zip(dq_ref.make_contiguous().iter())
            .for_each(|(a, &b)| {
                assert_eq!(a, b);
            });
    }
}
