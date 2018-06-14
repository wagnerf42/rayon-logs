//! We define here all traits enhancing parallel iterators.
use rayon::prelude::ParallelIterator;

use {Logged, LoggedPool};

/// This trait extends `ParallelItertor`s by providing logging facilities.
pub trait LoggedParallelIterator: ParallelIterator {
    /// Log all thread activities in the provided LoggedPool.
    fn log(self, pool: &LoggedPool) -> Logged<Self> {
        Logged::new(self, pool)
    }
}

impl<I: ParallelIterator> LoggedParallelIterator for I {}
