pub trait ExplicitCopy: Copy {
    #[inline(always)]
    fn copy(&self) -> Self {
        *self
    }
}

impl<T: Copy> ExplicitCopy for T {}
