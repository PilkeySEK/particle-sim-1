use std::ops::Deref;

#[derive(Copy, Clone)]
pub struct Immutable<T> {
    inner: T,
}

impl<T> Deref for Immutable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Immutable<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}
