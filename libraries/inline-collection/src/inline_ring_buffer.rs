use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Index, IndexMut};

pub struct InlineRingBuf<T, const N: usize> {
    buffer: ManuallyDrop<[T; N]>,
    len: usize,
    offset: usize,
}
impl<T, const N: usize> InlineRingBuf<T, N> {
    #[cfg(not(test))]
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
            offset: 0,
        }
    }
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::zeroed().assume_init() },
            len: 0,
            offset: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        N
    }
    pub fn push_head(&mut self, item: T) {
        if self.len >= self.capacity() {
            unsafe {
                let ptr = self.buffer.as_mut_ptr().add(self.offset);
                ptr.drop_in_place();
                ptr.write(item);
                self.increment_offset();
            }
            return;
        }

        unsafe {
            self.buffer.as_mut_ptr().add(self.index(self.len)).write(item);
        }
        self.len += 1;
    }

    pub fn pop_head(&mut self) -> Option<T> {
        if self.len <= 0 {
            return None;
        }

        self.len -= 1;
        let item = unsafe { self.buffer.as_mut_ptr().add(self.index(self.len)).read() };

        return Some(item);
    }

    pub fn push_tail(&mut self, item: T) {
        if self.len >= self.capacity() {
            unsafe {
                let ptr = self.buffer.as_mut_ptr().add(self.index(self.len - 1));
                ptr.drop_in_place();
                ptr.write(item);
                self.decrement_offset();
            }
            return;
        }

        self.decrement_offset();
        unsafe {
            self.buffer.as_mut_ptr().add(self.index(0)).write(item);
        }
        self.len += 1;
    }

    pub fn pop_tail(&mut self) -> Option<T> {
        if self.len <= 0 {
            return None;
        }

        let item = unsafe { self.buffer.as_mut_ptr().add(self.index(0)).read() };
        self.len -= 1;
        self.increment_offset();

        return Some(item);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len <= 0
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if self.len <= index {
            None
        } else {
            Some(&self.buffer[self.index(index)])
        }
    }
    pub fn iter(&self) -> InlineRingBufIter<T, N> {
        InlineRingBufIter { buf: self, index: 0 }
    }

    fn index(&self, index: usize) -> usize {
        (self.offset + index) % self.capacity()
    }

    fn increment_offset(&mut self) {
        let offset = self.offset + 1;
        if offset >= self.capacity() {
            self.offset = 0;
        } else {
            self.offset = offset;
        }
    }
    fn decrement_offset(&mut self) {
        if self.offset <= 0 {
            self.offset = self.capacity() - 1;
        } else {
            self.offset = self.offset - 1;
        }
    }
}

impl<T, const N: usize> Drop for InlineRingBuf<T, N> {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.buffer.as_mut_ptr();
            for i in 0..self.len {
                ptr.add(self.index(i)).drop_in_place();
            }
        }
    }
}

impl<T, const N: usize> Index<usize> for InlineRingBuf<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.buffer[self.index(index)]
    }
}
impl<T, const N: usize> IndexMut<usize> for InlineRingBuf<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let index = self.index(index);
        &mut self.buffer[index]
    }
}
impl<T, const N: usize> Default for InlineRingBuf<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> From<[T; N]> for InlineRingBuf<T, N> {
    fn from(item: [T; N]) -> Self {
        Self {
            buffer: ManuallyDrop::new(item),
            len: N,
            offset: 0,
        }
    }
}

pub struct InlineRingBufIter<'a, T, const N: usize> {
    buf: &'a InlineRingBuf<T, N>,
    index: usize,
}

impl<'a, T, const N: usize> Iterator for InlineRingBufIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.buf.get(self.index);
        if result.is_some() {
            self.index += 1;
        }
        return result;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.buf.len() - self.index;
        (rest, Some(rest))
    }
}

impl<'a, T, const N: usize> ExactSizeIterator for InlineRingBufIter<'a, T, N> {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::*;
    use crate::InlineRingBuf;
    use std::fmt::Debug;

    #[test]
    fn from() {
        validate([1, 2, 3]);
        validate([1, 2, 3, 4, 5]);

        fn validate<const N: usize, T: Clone + PartialEq>(source: [T; N]) {
            let buf = InlineRingBuf::from(source.clone());

            for i in 0..buf.len() {
                assert!(buf.get(i) == Some(&source[i]));
            }
        }
    }

    #[test]
    fn new() {
        validate::<3>();
        validate::<4>();
        validate::<5>();

        fn validate<const N: usize>() {
            let buf = InlineRingBuf::<DropMarker<i32>, N>::new();

            assert!(buf.len() == 0);
        }
    }

    #[test]
    fn push_head() {
        validate::<_, 3>(&[1, 2, 3, 4, 5]);
        validate::<_, 5>(&[1, 2, 3, 4, 5]);
        validate::<_, 7>(&[1, 2, 3, 4, 5]);

        fn validate<T: PartialEq + Debug + Clone, const N: usize>(source: &[T]) {
            let watcher = DropWatcher::new();
            {
                let mut buf = InlineRingBuf::<_, N>::new();
                let should_be_contained = source[source.len().saturating_sub(N)..].to_vec();

                for x in source {
                    let marker = watcher.alloc(x.clone());
                    buf.push_head(marker);
                }

                for (idx, item) in should_be_contained.iter().enumerate() {
                    assert!(buf.get(idx).any(|&m| &*m.props() == item))
                }
            }
            assert!(watcher.markers().iter().all(|s| s.is_properly_dropped()));
        }
    }

    #[test]
    fn push_tail() {
        validate::<_, 3>(&[1, 2, 3, 4, 5]);
        validate::<_, 5>(&[1, 2, 3, 4, 5]);
        validate::<_, 7>(&[1, 2, 3, 4, 5]);

        fn validate<T: PartialEq + Debug + Clone, const N: usize>(source: &[T]) {
            let watcher = DropWatcher::new();
            {
                let mut buf = InlineRingBuf::<_, N>::new();
                let should_be_contained = source[source.len().saturating_sub(N)..].to_vec();

                for x in source {
                    let marker = watcher.alloc(x.clone());
                    buf.push_tail(marker);
                }

                for (idx, item) in should_be_contained.iter().rev().enumerate() {
                    assert!(buf.get(idx).any(|&m| &*m.props() == item))
                }
            }
            assert!(watcher.markers().iter().all(|s| s.is_properly_dropped()));
        }
    }
}
