//! Per-category locks for re-rolling.
//!
//! A creator's lock toggles decide what a re-roll is allowed to touch: lock the
//! build, re-roll, and the body keeps its mass while everything else changes.
//!
//! Locks are stored as a bitmask so the field grows additively — a reader that
//! predates a category simply ignores its bit, which is what the AT Protocol's
//! schema-evolution rules require of every field.

use serde::{Deserialize, Serialize};

use crate::plan::Category;

/// Which categories a re-roll must leave alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LockSet {
    /// One bit per [`Category`], as given by [`Category::bit`].
    pub bits: u32,
}

impl LockSet {
    /// Nothing locked: a re-roll changes everything.
    pub const NONE: LockSet = LockSet { bits: 0 };

    /// Whether `category` is protected from re-rolling.
    #[must_use]
    pub fn is_locked(self, category: Category) -> bool {
        self.bits & category.bit() != 0
    }

    /// Locks `category`.
    #[must_use]
    pub fn with(mut self, category: Category) -> Self {
        self.bits |= category.bit();
        self
    }

    /// Unlocks `category`.
    #[must_use]
    pub fn without(mut self, category: Category) -> Self {
        self.bits &= !category.bit();
        self
    }

    /// Flips `category`'s lock.
    pub fn toggle(&mut self, category: Category) {
        self.bits ^= category.bit();
    }

    /// Whether every known category is locked, so a re-roll would do nothing.
    #[must_use]
    pub fn is_everything(self) -> bool {
        Category::ALL.iter().all(|&c| self.is_locked(c))
    }

    /// The locked categories, in creator-panel order.
    #[must_use]
    pub fn locked(self) -> Vec<Category> {
        Category::ALL
            .into_iter()
            .filter(|&c| self.is_locked(c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_are_independent() {
        let locks = LockSet::NONE.with(Category::Build).with(Category::Head);
        assert!(locks.is_locked(Category::Build));
        assert!(locks.is_locked(Category::Head));
        assert!(!locks.is_locked(Category::Stature));
        assert_eq!(locks.locked(), vec![Category::Build, Category::Head]);
    }

    #[test]
    fn toggling_round_trips() {
        let mut locks = LockSet::NONE;
        locks.toggle(Category::Frame);
        assert!(locks.is_locked(Category::Frame));
        locks.toggle(Category::Frame);
        assert_eq!(locks, LockSet::NONE);
    }

    #[test]
    fn everything_locked_is_detected() {
        let mut locks = LockSet::NONE;
        assert!(!locks.is_everything());
        for category in Category::ALL {
            locks = locks.with(category);
        }
        assert!(locks.is_everything());
    }

    #[test]
    fn unknown_bits_survive_a_round_trip() {
        // A future category's bit must not be dropped by an older reader.
        let raw = LockSet { bits: 1 << 20 };
        let json = serde_json::to_string(&raw).expect("serialises");
        assert_eq!(json, (1u32 << 20).to_string(), "stored as a bare number");
        let back: LockSet = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(back, raw);
    }
}
