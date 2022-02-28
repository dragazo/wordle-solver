use std::iter::FusedIterator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsizeBitSet(usize);
impl UsizeBitSet {
    pub fn new() -> Self {
        UsizeBitSet(0)
    }
    pub fn insert(&mut self, pos: usize) {
        self.0 |= 1 << pos;
    }
    pub fn remove(&mut self, pos: usize) {
        self.0 &= !(1 << pos);
    }
    pub fn contains(&self, pos: usize) -> bool {
        self.0 & (1 << pos) != 0
    }
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn is_subset(&self, other: &UsizeBitSet) -> bool {
        self.0 & !other.0 == 0
    }
    pub fn intersect_with(&mut self, other: &UsizeBitSet) {
        self.0 &= other.0;
    }
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
}

/// Iterates over the items of the bitset in ascending order.
pub struct Iter(usize);
impl Iterator for Iter {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            0 => None,
            v => Some({
                let res = v.trailing_zeros() as usize;
                self.0 &= v - 1;
                res
            }),
        }
    }
}
impl FusedIterator for Iter {}

impl IntoIterator for UsizeBitSet {
    type Item = usize;
    type IntoIter = Iter;
    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0)
    }
}

#[test]
fn test_usize_bitset() {
    let mut s = UsizeBitSet::new();
    assert_eq!(s.0, 0b0);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[]);
    assert!(!s.contains(1) && !s.contains(3) && !s.contains(0) && !s.contains(2) && !s.contains(25));
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);

    s.insert(0);
    assert_eq!(s.0, 0b1);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[0]);
    assert!(!s.contains(1) && !s.contains(3) && s.contains(0) && !s.contains(2) && !s.contains(25));
    assert!(!s.is_empty());
    assert_eq!(s.len(), 1);

    s.insert(3);
    assert_eq!(s.0, 0b1001);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[0, 3]);
    assert!(!s.contains(1) && s.contains(3) && s.contains(0) && !s.contains(2) && !s.contains(25));
    assert!(!s.is_empty());
    assert_eq!(s.len(), 2);

    s.insert(3);
    assert_eq!(s.0, 0b1001);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[0, 3]);
    assert!(!s.is_empty());
    assert_eq!(s.len(), 2);

    s.insert(1);
    assert_eq!(s.0, 0b1011);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[0, 1, 3]);
    assert!(s.contains(1) && s.contains(3) && s.contains(0) && !s.contains(2) && !s.contains(25));
    assert!(!s.is_empty());
    assert_eq!(s.len(), 3);

    s.remove(0);
    assert_eq!(s.0, 0b1010);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[1, 3]);
    assert!(s.contains(1) && s.contains(3) && !s.contains(0));
    assert!(!s.is_empty());
    assert_eq!(s.len(), 2);

    s.remove(0);
    assert_eq!(s.0, 0b1010);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[1, 3]);
    assert!(!s.is_empty());
    assert_eq!(s.len(), 2);

    s.remove(7);
    assert_eq!(s.0, 0b1010);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[1, 3]);
    assert!(!s.is_empty());
    assert_eq!(s.len(), 2);

    for i in 0..26 {
        s.insert(i);
    }
    assert_eq!(s.0, (1 << 26) - 1);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), (0..26).collect::<Vec<_>>());
    assert!(!s.is_empty());
    assert_eq!(s.len(), 26);

    s.clear();
    assert_eq!(s.0, 0);
    assert_eq!(s.into_iter().collect::<Vec<_>>(), &[]);
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);

    assert_eq!(UsizeBitSet(0b10110111001000101101001110110110).into_iter().collect::<Vec<_>>(),
        &[1, 2, 4, 5, 7, 8, 9, 12, 14, 15, 17, 21, 24, 25, 26, 28, 29, 31]);

    let mut p = UsizeBitSet(0b10110111001000101101001110110110);
    let mut q = UsizeBitSet(0b10110111001000101101001110110110);
    assert!(p.is_subset(&q) && q.is_subset(&p));
    q = UsizeBitSet(0b10110111001000111101001110110110);
    assert!(p.is_subset(&q) && !q.is_subset(&p));
    q = UsizeBitSet(0b10110011100101010011010101010011);
    assert!(!p.is_subset(&q) && !q.is_subset(&p));
    q.intersect_with(&p);
    assert_eq!(p.0, 0b10110111001000101101001110110110);
    assert_eq!(q.0, 0b10110011000000000001000100010010);
    assert!(!p.is_subset(&q) && q.is_subset(&p));
    p.intersect_with(&q);
    assert_eq!(p.0, 0b10110011000000000001000100010010);
    assert_eq!(q.0, 0b10110011000000000001000100010010);
    assert_eq!(p.0, q.0);
    assert!(p.is_subset(&q) && q.is_subset(&p));
    assert_eq!(p.into_iter().collect::<Vec<_>>(), &[1, 4, 8, 12, 24, 25, 28, 29, 31]);
    assert_eq!(q.into_iter().collect::<Vec<_>>(), &[1, 4, 8, 12, 24, 25, 28, 29, 31]);
    assert_eq!(p.len(), 9);
    assert_eq!(q.len(), 9);
}