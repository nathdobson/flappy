use core::ops::Range;

pub struct SlicePair<'a, T> {
    slice1: &'a [T],
    slice2: &'a [T],
}
impl<'a, T> Copy for SlicePair<'a, T> {}
impl<'a, T> Clone for SlicePair<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> SlicePair<'a, T> {
    pub fn new(slice1: &'a [T], slice2: &'a [T]) -> SlicePair<'a, T> {
        SlicePair { slice1, slice2 }
    }
    pub fn slice1(self) -> &'a [T] {
        self.slice1
    }
    pub fn slice2(self) -> &'a [T] {
        self.slice2
    }
    pub fn len(&self) -> usize {
        self.slice1.len() + self.slice2.len()
    }
    pub fn slices(&self) -> [&'a [T]; 2] {
        [self.slice1, self.slice2]
    }
    pub fn subslice(self, range: Range<usize>) -> Option<Self> {
        if range.start < self.slice1.len() {
            if range.end < self.slice1.len() {
                Some(Self::new(self.slice1.get(range)?, &[]))
            } else {
                Some(Self::new(
                    self.slice1.get(range.start..)?,
                    self.slice2.get(..range.end - self.slice1.len())?,
                ))
            }
        } else {
            Some(Self::new(
                &[],
                self.slice2
                    .get(range.start - self.slice1.len()..range.end - self.slice1.len())?,
            ))
        }
    }
}

impl<'a, T> IntoIterator for SlicePair<'a, T> {
    type Item = &'a T;
    type IntoIter = impl 'a + Iterator<Item = &'a T>;
    fn into_iter(self) -> Self::IntoIter {
        self.slice1.iter().chain(self.slice2.iter())
    }
}
