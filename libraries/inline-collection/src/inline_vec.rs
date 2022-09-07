use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

pub struct InlineVec<T, const N: usize> {
    buffer: ManuallyDrop<[T; N]>,
    len: usize,
}
impl<T, const N: usize> InlineVec<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        N
    }
    pub fn push(&mut self, item: T) -> bool {
        if self.len >= self.capacity() {
            return false;
        }
        unsafe {
            let ptr = self.buffer.as_mut_ptr();
            ptr.add(self.len).write(item);
        }
        self.len += 1;

        return true;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len <= 0 {
            return None;
        }

        self.len -= 1;
        let item = unsafe { self.buffer.as_mut_ptr().add(self.len).read() };

        return Some(item);
    }

    pub fn peek(&self) -> Option<&T> {
        if self.len <= 0 {
            return None;
        }

        let item = unsafe { &*self.buffer.as_ptr().add(self.len - 1) };

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
            Some(&self.buffer[index])
        }
    }
}

impl<T, const N: usize> Drop for InlineVec<T, N> {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.buffer.as_mut_ptr();
            for i in 0..self.len {
                ptr.add(i).drop_in_place();
            }
        }
    }
}

impl<T, const N: usize> Deref for InlineVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self[..]
    }
}

impl<T, const N: usize> DerefMut for InlineVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

impl<I: SliceIndex<[T]>, T, const N: usize> Index<I> for InlineVec<T, N> {
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output {
        &self.buffer[..self.len][index]
    }
}
impl<I: SliceIndex<[T]>, T, const N: usize> IndexMut<I> for InlineVec<T, N> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.buffer[..self.len][index]
    }
}
impl<T, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
