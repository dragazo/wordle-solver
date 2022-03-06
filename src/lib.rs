use std::collections::BTreeSet;
use std::{iter, fmt};
use std::sync::{Arc, Mutex};
use std::ops::Deref;

use itertools::Itertools;
use float_ord::FloatOrd;

mod bit_set;
use bit_set::BitSet32;

#[derive(Debug)]
pub enum InputError<'a> {
    WrongHintLen { hint: &'a [Hint], expected_len: usize },
    WrongWordLen { word: &'a str, expected_len: usize },
    NotLowerAlpha { word: &'a str },
}

#[derive(Debug)]
pub enum SolveErr {
    Inconsistent
}

fn check_word(expected_len: usize, word: &str) -> Result<(), InputError> {
    if word.len() != expected_len {
        return Err(InputError::WrongWordLen { word, expected_len })
    }
    if word.chars().any(|c| !('a'..='z').contains(&c)) {
        return Err(InputError::NotLowerAlpha { word })
    }
    Ok(())
}

/// A set of valid, uniform-length words for a [`Puzzle`].
#[derive(Clone)]
pub struct Dictionary {
    data: Vec<u8>,
    word_len: usize,
}
impl Dictionary {
    /// Creates a new dictionary of words where each word is the specified `word_len`.
    /// If a word is invalid (incorrect length or not lowercase alphabetic), returns [`Err`].
    /// Panics if `word_len` is zero.
    pub fn with_words<'a, T: IntoIterator<Item = &'a str>>(word_len: usize, words: T) -> Result<Self, InputError<'a>> {
        assert!(word_len > 0);

        let words: BTreeSet<_> = words.into_iter().collect(); // sort and dedupe

        let mut data = vec![];
        for word in words {
            check_word(word_len, word)?;
            data.extend(word.as_bytes().iter().map(|&x| x - 97));
        }

        assert_eq!(data.len() % word_len, 0);
        Ok(Dictionary { data, word_len })
    }
    fn to_words(&self) -> Vec<Word> {
        self.data.chunks_exact(self.word_len).map(Word).collect()
    }
}

struct OwnedWord(Vec<u8>);
impl OwnedWord {
    fn new(expected_len: usize, word: &str) -> Result<Self, InputError> {
        check_word(expected_len, word)?;
        Ok(OwnedWord(word.as_bytes().iter().map(|&c| c - 97).collect()))
    }
}
impl OwnedWord {
    fn as_ref(&self) -> Word {
        Word(self.0.as_slice())
    }
}
impl Deref for OwnedWord {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Word<'a>(&'a [u8]);
impl<'a> Deref for Word<'a> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Hint { Correct, Present, Absent }

/// A wordle-like puzzle.
#[derive(Clone)]
pub struct Puzzle<'a> {
    all_words: Arc<Vec<Word<'a>>>,

    slots: Vec<BitSet32>,
    letter_counts: [(usize, usize); 26],
}
impl<'a> Puzzle<'a> {
    /// Creates a new puzzle from a [`Dictionary`] of acceptable words to guess.
    /// This object does not store the answer to the puzzle, and is instead used as a solver state.
    /// The number of letters in the puzzle is defined by the supplied dictionary.
    pub fn new(dictionary: &'a Dictionary) -> Self {
        let all_words = Arc::new(dictionary.to_words());

        let mut allowed = BitSet32::new();
        for i in 0..26 { allowed.insert(i); }
        let mut res = Puzzle {
            all_words,
            slots: vec![allowed; dictionary.word_len],
            letter_counts: [(0, dictionary.word_len); 26],
        };
        res.reduce();
        res
    }
    fn could_be(&self, word: Word) -> bool {
        debug_assert!(word.len() == self.slots.len());

        let mut occurrences = [0; 26];
        for (slot, &letter) in iter::zip(&self.slots, word.iter()) {
            if !slot.contains(letter) { return false }
            occurrences[letter as usize] += 1;
        }
        for (counts, occ) in iter::zip(&self.letter_counts, occurrences) {
            if occ != 0 && !(counts.0..=counts.1).contains(&occ) { return false }
        }
        true
    }
    fn reduce(&mut self) {
        let mut masks = vec![BitSet32::new(); self.slots.len()];
        let mut slot_idxs = Vec::with_capacity(self.slots.len());

        loop {
            let mut did_something = false;

            // do slot-wise letter elimination by intersect with union over valid words
            for mask in masks.iter_mut() { mask.clear(); }
            for &word in self.all_words.iter() {
                if !self.could_be(word) { continue }
                for (mask, &letter) in iter::zip(&mut masks, word.iter()) {
                    mask.insert(letter);
                }
            }
            for (slot, mask) in iter::zip(&mut self.slots, &masks) {
                let prev = *slot;
                slot.intersect_with(mask);
                if *slot != prev { did_something = true; }
            }

            // do occurrence-based eliminations for slots with known occurrences
            for (letter, &(min, _)) in self.letter_counts.iter().enumerate() {
                let letter = letter as u8;

                slot_idxs.clear();
                slot_idxs.extend(self.slots.iter().enumerate().filter_map(|(i, slot)| if slot.contains(letter) { Some(i) } else { None }));
                if slot_idxs.len() > min { continue }

                for &idx in slot_idxs.iter() {
                    let slot = &mut self.slots[idx];
                    let prev = *slot;
                    slot.clear();
                    slot.insert(letter);
                    if *slot != prev { did_something = true; }
                }
            }

            if !did_something { return }
        }
    }
    fn guess_impl(&mut self, word: Word, response: &[Hint]) {
        debug_assert!(word.len() == response.len() && word.len() == self.slots.len());

        // (slot, (letter, hint)) -- sorted by letter, then by hint, then by slot
        let mut word: Vec<(usize, (u8, Hint))> = iter::zip(word.iter().copied(), response.iter().copied()).enumerate().collect();
        word.sort_by_key(|x| (x.1.0, match x.1.1 { Hint::Correct => 0, Hint::Present => 1, Hint::Absent => 2 }, x.0));

        let mut prev_char = 0;
        let mut occ_idx = 0;
        for (i, (ch, hint)) in word.iter().copied() {
            if ch != prev_char { occ_idx = 0; }

            let mut letter_counts = &mut self.letter_counts[ch as usize];
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
    /// Performs the solve state reductions corresponding to guessing the given word and receiving the supplied hint from the game.
    /// The `word` is assumed to be a valid word from the dictionary, but this is not enforced.
    /// If the `word` is invalid (not lower alphabetic or wrong length), or if the hint is the wrong length, returns [`Err`].
    pub fn guess<'b>(&mut self, word: &'b str, hint: &'b [Hint]) -> Result<(), InputError<'b>> {
        let word = OwnedWord::new(self.slots.len(), word)?;
        if word.len() != hint.len() { return Err(InputError::WrongHintLen { hint, expected_len: self.slots.len() }); }
        self.guess_impl(word.as_ref(), hint);
        Ok(())
    }
    /// From the set of all valid words in the dictionary used to construct the object,
    /// finds the word which has the best worst-case (over the set of consistent hints) number of possible solutions after using it as a guess.
    /// In the event of ties, the word with the best average-case is selected, and further ties are broken by taking the first word in the lexicographic ordering.
    /// If there are no possible solutions (an inconsistent puzzle), returns [`Err`].
    /// Returns a tuple `(word, worst_case_remaining, avg_case_remaining)`.
    /// 
    /// Because this logic can be slow, it is performed in parallel over all the words in the dictionary.
    /// The `threads` input specifies the number of threads to use.
    /// If `threads` is zero, it is defaulted to `1`.
    pub fn best_guess(&self, mut threads: usize) -> Result<(String, u64, f64), SolveErr> {
        if self.slots.iter().any(BitSet32::is_empty) {
            return Err(SolveErr::Inconsistent);
        }
        if self.slots.iter().all(|s| s.len() == 1) {
            return Ok((self.slots.iter().map(|&s| char::from_u32(s.into_iter().next().unwrap() as u32 + 97).unwrap()).collect(), 0, 0.0));
        }
        threads = threads.max(1);

        let best = crossbeam::scope(|scope| {
            let guesses = Arc::new(Mutex::new(self.all_words.iter().copied().fuse()));
            let threads: Vec<_> = (0..threads).map(|_| {
                let guesses = guesses.clone();
                let this = self.clone();
                scope.spawn(move |_| {
                    let mut best: Option<(Word, (u64, FloatOrd<f64>), bool)> = None; // (guess, (worst case remaining, avg case remaining), could be answer flag)
                    'next_word: loop {
                        let guess = match guesses.lock().unwrap().next() {
                            Some(x) => x,
                            None => break,
                        };

                        let mut worst: u64 = 0;
                        let mut worst_avg: (u64, u64) = (0, 0);
                        'next_response: for response in iter::once([Hint::Absent, Hint::Present, Hint::Correct]).cycle().take(this.slots.len()).multi_cartesian_product() {
                            let mut cpy = this.clone();
                            cpy.guess_impl(guess, &response);
                            if cpy.slots.iter().any(BitSet32::is_empty) { continue 'next_response; }
                            let possible = self.all_words.iter().filter(|&&s| cpy.could_be(s)).count() as u64;
                            if possible == 0 { continue 'next_response; }

                            worst = worst.max(possible);
                            worst_avg.0 += possible;
                            worst_avg.1 += 1;

                            if let Some(prev) = best {
                                if worst > prev.1.0 { continue 'next_word; }
                            }
                        }
                        if worst == 0 { continue 'next_word; }
                        debug_assert_ne!(worst_avg.1, 0);

                        let score = (worst, FloatOrd(worst_avg.0 as f64 / worst_avg.1 as f64));
                        let replace = match best {
                            None => true,
                            Some(prev) => score < prev.1 || (score == prev.1 && !prev.2),
                        };
                        if replace { best = Some((guess, score, this.could_be(guess))); }
                    }
                    best
                })
            }).collect();

            threads.into_iter().filter_map(|t| t.join().unwrap()).min_by_key(|&(guess, score, cbf)| (score, if cbf { 0 } else { 1 }, guess))
        }).unwrap();

        match best {
            Some(x) => Ok((x.0.iter().map(|&c| char::from_u32(c as u32 + 97).unwrap()).collect(), x.1.0, x.1.1.0)),
            None => Err(SolveErr::Inconsistent),
        }
    }
}
impl fmt::Display for Puzzle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let letters = "abcdefghijklmnopqrstuvwxyz";
        let mut mapped = BTreeSet::new();

        for (i, &slot) in self.slots.iter().enumerate() {
            mapped.clear();
            for v in slot { mapped.insert(&letters[v as usize..v as usize + 1]); }
            let txt = mapped.iter().fold(String::new(), |acc, v| acc + v);
            writeln!(f, "{}: {}", i, txt)?;
        }

        write!(f, "{{ ").unwrap();
        for (counts, letter) in iter::zip(&self.letter_counts, letters.chars()) {
            write!(f, "{}: {}..={}, ", letter, counts.0, counts.1).unwrap();
        }
        writeln!(f, "}}").unwrap();

        Ok(())
    }
}