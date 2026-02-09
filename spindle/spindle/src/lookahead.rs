use core::str::CharIndices;
use heapless::Deque;

pub struct Lookahead<const N: usize, I: Iterator> {
    buffer: Deque<I::Item, N>,
    inner: I,
}

impl<const N: usize, I: Iterator> Lookahead<N, I> {
    pub fn new(inner: I) -> Self {
        Lookahead {
            buffer: Deque::new(),
            inner,
        }
    }
    pub fn peek(&mut self, index: usize) -> Option<&I::Item> {
        assert!(index < self.buffer.capacity());
        while self.buffer.len() <= index {
            self.buffer.push_back(self.inner.next()?).ok().unwrap();
        }
        Some(self.buffer.get(index).unwrap())
    }
}

impl<const N: usize, I: Iterator> Iterator for Lookahead<N, I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(result) = self.buffer.pop_front() {
            Some(result)
        } else {
            self.inner.next()
        }
    }
}
