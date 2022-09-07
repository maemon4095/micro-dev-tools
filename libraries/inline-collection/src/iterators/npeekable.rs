use crate::inline_ring_buffer::*;
pub struct NPeekable<I: Iterator, const N: usize> {
    iter: I,
    peeked: InlineRingBuf<I::Item, N>,
}

impl<const N: usize, I: Iterator> NPeekable<I, N> {
    pub fn peek(&mut self) -> Option<&I::Item> {
        self.nth_peek(0)
    }

    pub fn nth_peek(&mut self, index: usize) -> Option<&I::Item> {
        if self.peeked.capacity() <= index {
            return None;
        }
        self.load(index + 1);
        self.peeked.get(index)
    }

    pub fn peek_range(&mut self, count: usize) -> std::iter::Take<InlineRingBufIter<'_, I::Item, N>> {
        self.load(count);
        self.peeked.iter().take(count)
    }

    pub fn has_less_than(&mut self, count: usize) -> bool {
        self.peek_range(count).len() < count
    }
    pub fn has_less_than_or_eq(&mut self, count: usize) -> bool {
        self.peek_range(count).len() <= count
    }

    pub fn has_exact(&mut self, count: usize) -> Option<bool> {
        let peeked = self.peek_range(count);
        if count >= N {
            return if peeked.len() < count { Some(false) } else { None };
        }

        Some(peeked.len() == count)
    }

    pub fn has_exact_and(&mut self, count: usize, predicate: impl FnMut((usize, &I::Item)) -> bool) -> Option<bool> {
        let mut peeked = self.peek_range(count + 1).enumerate();
        if count >= N {
            return if peeked.len() < count || !peeked.all(predicate) { Some(false) } else { None };
        }
        Some(peeked.len() == count && peeked.all(predicate))
    }

    pub fn starts(&mut self, count: usize, predicate: impl FnMut((usize, &I::Item)) -> bool) -> Option<bool> {
        let mut peeked = self.peek_range(count).enumerate();
        if count >= N {
            if peeked.len() < count || !peeked.all(predicate) {
                return Some(false);
            }
            return None;
        }
        Some(peeked.all(predicate))
    }

    fn load(&mut self, count: usize) {
        while self.peeked.len() < N.min(count) {
            if let Some(item) = self.iter.next() {
                self.peeked.push_head(item);
            } else {
                return;
            }
        }
    }
}

impl<I: Iterator, const N: usize> From<I> for NPeekable<I, N> {
    fn from(iter: I) -> Self {
        Self {
            iter,
            peeked: InlineRingBuf::new(),
        }
    }
}

impl<const N: usize, I: Iterator> Iterator for NPeekable<I, N> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.peeked.pop_tail().or_else(|| self.iter.next())
    }
}

#[cfg(test)]
mod tests {
    use crate::iterators::IteratorExtensions;

    #[test]
    fn nth_peek() {
        validate::<4>(2);
        validate::<5>(5);
        validate::<2>(5);

        fn validate<const N: usize>(count: usize) {
            let mut peekable = (0..count).into_iter().n_peekable::<N>();

            for i in 0..N.min(count) {
                assert!(peekable.nth_peek(i) == Some(&i));
            }

            for i in 0..count {
                assert!(peekable.next() == Some(i));
            }
        }
    }

    #[test]
    fn has_less_than() {
        validate::<4>(2);
        validate::<5>(5);
        validate::<2>(5);

        fn validate<const N: usize>(count: usize) {
            let mut peekable = (0..count).into_iter().n_peekable::<N>();
            let should_contains = N.min(count);
            assert!((0..=should_contains).all(|c| !peekable.has_less_than(c)));
            assert!(((should_contains + 1)..(should_contains * 2)).all(|c| peekable.has_less_than(c)));
        }
    }

    #[test]
    fn has_exact() {
        validate::<4>(2);
        validate::<5>(5);
        validate::<2>(5);

        fn validate<const N: usize>(count: usize) {
            let mut peekable = (0..count).into_iter().n_peekable::<N>();
            if N > count {
                assert!((0..count).all(|i| peekable.has_exact(i) == Some(false)));
                assert!(((count + 1)..N).all(|i| peekable.has_exact(i) == Some(false)));
                assert!(peekable.has_exact(count) == Some(true));
            } else {
                assert!((0..N).all(|i| peekable.has_exact(i) == Some(false)));
                assert!(peekable.has_exact(N) == None);
            }
        }
    }
}
