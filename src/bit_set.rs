use std::iter::FusedIterator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitSet32(u32);
impl BitSet32 {
    pub fn new() -> Self {
        BitSet32(0)
    }
    pub fn insert(&mut self, pos: u8) {
        self.0 |= 1 << pos;
    }
    pub fn remove(&mut self, pos: u8) {
        self.0 &= !(1 << pos);
    }
    pub fn contains(&self, pos: u8) -> bool {
        self.0 & (1 << pos) != 0
    }
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn intersect_with(&mut self, other: &BitSet32) {
        self.0 &= other.0;
    }
    pub fn len(&self) -> u32 {
        self.0.count_ones()
    }
}

/// Iterates over the items of the bitset in ascending order.
pub struct Iter(u32);
impl Iterator for Iter {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            0 => None,
            v => Some({
                let res = v.trailing_zeros() as u8;
                self.0 &= v - 1;
                res
            }),
        }
    }
}
impl FusedIterator for Iter {}

impl IntoIterator for BitSet32 {
    type Item = u8;
    type IntoIter = Iter;
    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0)
    }
}

#[test]
fn test_bitset32() {
    let mut s = BitSet32::new();
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

    assert_eq!(BitSet32(0b10110111001000101101001110110110).into_iter().collect::<Vec<_>>(),
        &[1, 2, 4, 5, 7, 8, 9, 12, 14, 15, 17, 21, 24, 25, 26, 28, 29, 31]);

    let mut p = BitSet32(0b10110111001000101101001110110110);
    let mut q = BitSet32(0b10110011100101010011010101010011);
    q.intersect_with(&p);
    assert_eq!(p.0, 0b10110111001000101101001110110110);
    assert_eq!(q.0, 0b10110011000000000001000100010010);
    p.intersect_with(&q);
    assert_eq!(p.0, 0b10110011000000000001000100010010);
    assert_eq!(q.0, 0b10110011000000000001000100010010);
    assert_eq!(p.0, q.0);
    assert_eq!(p.into_iter().collect::<Vec<_>>(), &[1, 4, 8, 12, 24, 25, 28, 29, 31]);
    assert_eq!(q.into_iter().collect::<Vec<_>>(), &[1, 4, 8, 12, 24, 25, 28, 29, 31]);
    assert_eq!(p.len(), 9);
    assert_eq!(q.len(), 9);
}