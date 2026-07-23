//! Canonical fixed-width logical CPU masks.
//!
//! Four words cover the architecture profile's 256 logical CPU limit. All
//! operations accept validated `CpuIndex` values, so bits outside the profile
//! cannot enter a mask.

use super::{CpuIndex, MAX_CPU_COUNT};

const MASK_WORD_BITS: usize = u64::BITS as usize;
pub const CPU_MASK_WORD_COUNT: usize = MAX_CPU_COUNT / MASK_WORD_BITS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuMask {
    words: [u64; CPU_MASK_WORD_COUNT],
}

impl CpuMask {
    pub const fn empty() -> Self {
        Self {
            words: [0; CPU_MASK_WORD_COUNT],
        }
    }

    pub fn singleton(index: CpuIndex) -> Self {
        let mut mask = Self::empty();
        mask.insert(index);
        mask
    }

    pub fn insert(&mut self, index: CpuIndex) -> bool {
        let (word, bit) = location(index);
        let previous = self.words[word];
        self.words[word] |= bit;
        previous & bit == 0
    }

    pub fn remove(&mut self, index: CpuIndex) -> bool {
        let (word, bit) = location(index);
        let previous = self.words[word];
        self.words[word] &= !bit;
        previous & bit != 0
    }

    pub const fn contains(self, index: CpuIndex) -> bool {
        let raw = index.as_usize();
        self.words[raw / MASK_WORD_BITS] & (1u64 << (raw % MASK_WORD_BITS)) != 0
    }

    pub fn count(self) -> u16 {
        self.words.iter().map(|word| word.count_ones() as u16).sum()
    }

    pub fn first(self) -> Option<CpuIndex> {
        for (word_index, word) in self.words.iter().copied().enumerate() {
            if word != 0 {
                let raw = word_index * MASK_WORD_BITS + word.trailing_zeros() as usize;
                return CpuIndex::new(raw as u16);
            }
        }
        None
    }

    pub fn union(self, other: Self) -> Self {
        self.combine(other, |left, right| left | right)
    }

    pub fn intersection(self, other: Self) -> Self {
        self.combine(other, |left, right| left & right)
    }

    pub fn difference(self, other: Self) -> Self {
        self.combine(other, |left, right| left & !right)
    }

    pub fn is_subset_of(self, other: Self) -> bool {
        self.words
            .iter()
            .zip(other.words.iter())
            .all(|(left, right)| left & !right == 0)
    }

    pub const fn is_empty(self) -> bool {
        let mut index = 0;
        while index < CPU_MASK_WORD_COUNT {
            if self.words[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }

    pub const fn from_words(words: [u64; CPU_MASK_WORD_COUNT]) -> Self {
        Self { words }
    }

    pub const fn words(self) -> [u64; CPU_MASK_WORD_COUNT] {
        self.words
    }

    fn combine(self, other: Self, operation: impl Fn(u64, u64) -> u64) -> Self {
        let mut words = [0; CPU_MASK_WORD_COUNT];
        for (index, output) in words.iter_mut().enumerate() {
            *output = operation(self.words[index], other.words[index]);
        }
        Self { words }
    }
}

impl Default for CpuMask {
    fn default() -> Self {
        Self::empty()
    }
}

fn location(index: CpuIndex) -> (usize, u64) {
    let raw = index.as_usize();
    (raw / MASK_WORD_BITS, 1u64 << (raw % MASK_WORD_BITS))
}
