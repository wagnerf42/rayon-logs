//! We define here all traits enhancing parallel iterators.
use rayon::prelude::IntoParallelRefIterator;
pub use rayon::prelude::ParallelIterator;

use Logged;

/// This trait extends `IntoParallelRefIterator`s by providing logging facilities.
pub trait IntoLoggedParallelRefIterator<'data>: IntoParallelRefIterator<'data> {
    /// Get a parallel logging iterator.
    fn par_iter(&'data self) -> Logged<Self::Iter> {
        Logged::new(IntoParallelRefIterator::par_iter(self))
    }
}

impl<'data, I: IntoParallelRefIterator<'data>> IntoLoggedParallelRefIterator<'data> for I {}
