use std::ops::{IndexMut, Index};
use std::collections::VecDeque;


#[derive(Default)]
pub struct TightVec<T> {
    buf: Vec<T>,
    next: VecDeque<usize>,
}

impl<T> Index<usize> for TightVec<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        &self.buf[index]
    }
}

impl<T> IndexMut<usize> for TightVec<T> {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.buf[index]
    }
}

impl<T> TightVec<T> {
    pub fn insert(&mut self, value: T) {
        match self.next.pop_front() {
            Some(index) => {
                self.buf[index] = value;
            }
            None => {
                self.buf.push(value);
            }
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn used_len(&self) -> usize {
        self.len() - self.unused_len()
    }

    #[inline(always)]
    pub fn unused_len(&self) -> usize {
        self.next.len()
    }

    pub fn replace_with(&mut self, index: usize, replace: T) {
        if self.buf.len() > index {
            self.buf[index] = replace;
            self.next.push_back(index);
        }
    }
}

impl<T: Default> TightVec<T> {
    pub fn remove(&mut self, index: usize) {
        if self.buf.len() > index {
            self.buf[index] = T::default();
            self.next.push_back(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Eq, PartialEq, Debug)]
    enum SimpleData {
        Empty,
        Data1,
        Data2,
    }
    impl Default for SimpleData {
        fn default() -> Self {
            SimpleData::Empty
        }
    }

    #[test]
    fn insert_works_like_normal_pushing() {
        let mut t = TightVec::default();
        t.insert(2.0);
        assert_eq!(t.len(), 1);
        assert_eq!(t.used_len(), 1);
        assert_eq!(t.unused_len(), 0);
    }

    #[test]
    fn can_remove_and_fill_empty_slots() {
        let mut t = TightVec::default();
        t.insert(SimpleData::Data1);
        t.insert(SimpleData::Data1);
        t.insert(SimpleData::Data1);
        assert_eq!(t.len(), 3);

        t.remove(1);
        assert_eq!(&t[1], &SimpleData::Empty);
        assert_eq!(t.len(), 3);
        assert_eq!(t.used_len(), 2);
        assert_eq!(t.unused_len(), 1);

        t.insert(SimpleData::Data2);
        assert_eq!(&t[1], &SimpleData::Data2);
    }

    #[test]
    fn remove_cant_panic() {
        let mut t = TightVec::default();
        t.insert(true);

        t.remove(0);
        t.remove(0);
        t.remove(1);
        t.remove(100000);
    }
}
