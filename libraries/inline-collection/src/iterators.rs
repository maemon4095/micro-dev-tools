mod npeekable;
pub use npeekable::NPeekable;
pub trait IteratorExtensions<I: Iterator> {
    fn n_peekable<const N: usize>(self) -> NPeekable<I, N>;
}

impl<I: Iterator> IteratorExtensions<I> for I {
    fn n_peekable<const N: usize>(self) -> NPeekable<I, N> {
        NPeekable::from(self)
    }
}
