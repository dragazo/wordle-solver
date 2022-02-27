use std::collections::{BTreeSet, BTreeMap};
use std::{iter, fmt};
use std::sync::{Arc, Mutex};
use std::ops::Deref;

use bit_set::BitSet;
use itertools::Itertools;

#[macro_use] extern crate lazy_static;

#[derive(Debug)] pub enum GuessErr { InvalidInput }
#[derive(Debug)] pub enum SolveErr { Inconsistent }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct CheckedStr<'a>(&'a str);
impl<'a> Deref for CheckedStr<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl<'a> CheckedStr<'a> {
    fn new(raw: &'a str, expected_len: usize) -> Result<Self, GuessErr> {
        match raw.len() == expected_len && raw.chars().all(|x| ('a'..='z').contains(&x)) {
            true => Ok(Self(raw)),
            false => Err(GuessErr::InvalidInput),
        }
    }
}

lazy_static! {
    static ref WORD_LIST: BTreeMap<usize, Vec<CheckedStr<'static>>> = {
        let mut res: BTreeMap<usize, BTreeSet<CheckedStr>> = Default::default();
        for raw in include_str!("guess-list.txt").lines().map(str::trim).filter(|s| s.len() != 0) {
            let word = CheckedStr::new(raw, raw.len()).unwrap();
            res.entry(word.len()).or_default().insert(word);
        }
        res.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect()
    };
}

#[derive(Debug, Clone, Copy)]
pub enum Hint { Correct, Present, Absent }

#[derive(Debug, Clone)]
pub struct Puzzle {
    slots: Vec<BitSet<u32>>,
    letter_counts: [(usize, usize); 26],
}
impl Puzzle {
    pub fn new(length: usize) -> Self {
        let mut allowed = BitSet::new();
        for i in 0..26 { allowed.insert(i); }
        let mut res = Puzzle {
            slots: vec![allowed; length],
            letter_counts: [(0, length); 26],
        };
        res.reduce();
        res
    }
    fn could_be(&self, word: CheckedStr) -> bool {
        debug_assert!(word.len() == self.slots.len());

        let mut occurrences = [0; 26];
        for (slot, letter) in iter::zip(&self.slots, word.as_bytes().iter().map(|&x| x as usize - 97)) {
            if !slot.contains(letter) { return false }
            occurrences[letter] += 1;
        }
        for (counts, occ) in iter::zip(&self.letter_counts, occurrences) {
            if occ != 0 && !(counts.0..=counts.1).contains(&occ) { return false }
        }
        true
    }
    fn reduce(&mut self) {
        let mut masks = vec![BitSet::new(); self.slots.len()];
        for &word in WORD_LIST.get(&self.slots.len()).map(|x| x.as_slice()).unwrap_or(&[]) {
            if !self.could_be(word) { continue }
            for (mask, &ch) in iter::zip(&mut masks, word.as_bytes()) {
                mask.insert(ch as usize - 97);
            }
        }
        for (slot, mask) in iter::zip(&mut self.slots, &masks) {
            slot.intersect_with(mask)
        }
    }
    fn guess_impl(&mut self, word: CheckedStr, response: &[Hint]) {
        debug_assert!(word.len() == response.len() && word.len() == self.slots.len());

        // (slot, (letter, hint)) -- sorted by letter, then by hint, then by slot
        let mut word: Vec<(usize, (usize, Hint))> = word.as_bytes().iter().map(|&x| x as usize - 97).zip(response.iter().copied()).enumerate().collect();
        word.sort_by_key(|x| (x.1.0, match x.1.1 { Hint::Correct => 0, Hint::Present => 1, Hint::Absent => 2 }, x.0));

        let mut prev_char = 0;
        let mut occ_idx = 0;
        for (i, (ch, hint)) in word.iter().copied() {
            if ch != prev_char { occ_idx = 0; }

            let mut letter_counts = &mut self.letter_counts[ch];
            let slot = &mut self.slots[i];
            match hint {
                Hint::Correct => {
                    letter_counts.0 = letter_counts.0.max(occ_idx + 1);
                    slot.clear();
                    slot.insert(ch);
                }
                Hint::Present => {
                    letter_counts.0 = letter_counts.0.max(occ_idx + 1);
                    slot.remove(ch);
                }
                Hint::Absent => {
                    letter_counts.1 = letter_counts.1.min(occ_idx);
                    if occ_idx == 0 {
                        for slot in self.slots.iter_mut() {
                            slot.remove(ch);
                        }
                    }
                }
            }

            prev_char = ch;
            occ_idx += 1;
        }

        self.reduce();
    }
    pub fn guess(&mut self, word: &str, response: &[Hint]) -> Result<(), GuessErr> {
        let word = CheckedStr::new(word, self.slots.len())?;
        if word.len() != response.len() { return Err(GuessErr::InvalidInput); }
        Ok(self.guess_impl(word, response))
    }
    pub fn best_guess(&self, mut threads: usize) -> Result<(&'static str, usize), SolveErr> {
        if self.slots.iter().any(BitSet::is_empty) { return Err(SolveErr::Inconsistent); }
        threads = threads.max(1);

        let word_pool = WORD_LIST.get(&self.slots.len()).map(|x| x.as_slice()).unwrap_or(&[]);

        // all the guessing words that could yield SOME amount of information 
        // let feasible_pool = Arc::new(word_pool.iter().copied().filter(|&guess| {
        //     let total_mask: BitSet = guess.as_bytes().iter().map(|&x| x as usize - 97).collect();
        //     self.slots.iter().any(|slot| !slot.is_disjoint(&total_mask))
        //     // true
        // }).collect::<Vec<_>>());
        // println!("pool size: {}", feasible_pool.len());
        // let guesses = Arc::new(Mutex::new(feasible_pool.deref().clone().into_iter().fuse()));


        let guesses = Arc::new(Mutex::new(word_pool.iter().copied().fuse()));
        let threads: Vec<_> = (0..threads).map(|_| {
            let guesses = guesses.clone();
            let this = self.clone();
            std::thread::spawn(move || {
                let mut best: Option<(CheckedStr, usize, bool)> = None; // (guess, remaining words, could be answer flag)
                'next_word: loop {
                    let guess = match guesses.lock().unwrap().next() {
                        Some(x) => x,
                        None => break,
                    };

                    let mut worst = 0;
                    for response in iter::once([Hint::Absent, Hint::Present, Hint::Correct]).cycle().take(this.slots.len()).multi_cartesian_product() {
                        let mut cpy = this.clone();
                        cpy.guess_impl(guess, &response);
                        let possible = word_pool.iter().filter(|&&s| cpy.could_be(s)).count();
                        worst = worst.max(possible);

                        if let Some(prev) = best {
                            if worst > prev.1 { continue 'next_word; }
                        }
                    }
                    if worst == 0 { continue }

                    let replace = match best {
                        None => true,
                        Some(prev) => worst < prev.1 || (worst == prev.1 && !prev.2),
                    };
                    if replace { best = Some((guess, worst, this.could_be(guess))); }
                }
                best
            })
        }).collect();

        let best = threads.into_iter().filter_map(|t| t.join().unwrap()).min_by_key(|&(guess, val, cbf)| (val, if cbf { 0 } else { 1 }, guess));
        match best {
            Some(x) => Ok((x.0.0, x.1)),
            None => Err(SolveErr::Inconsistent),
        }
    }
}
impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let letters = "abcdefghijklmnopqrstuvwxyz";
        let mut mapped = BTreeSet::new();

        for (i, slot) in self.slots.iter().enumerate() {
            mapped.clear();
            for v in slot { mapped.insert(&letters[v..v+1]); }
            let txt = mapped.iter().fold(String::new(), |acc, v| acc + v);
            writeln!(f, "{}: {}", i, txt)?;
        }
        Ok(())
    }
}