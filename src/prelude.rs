//! We define here all traits enhancing parallel iterators.
pub use rayon::prelude::ParallelIterator;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};

use Logged;

/// This trait extends `IntoParallelRefIterator`s by providing logging facilities.
pub trait IntoLoggedParallelRefIterator<'data>: IntoParallelRefIterator<'data> {
    /// Get a parallel logging iterator.
    fn par_iter(&'data self) -> Logged<Self::Iter> {
        Logged::new(IntoParallelRefIterator::par_iter(self))
    }
}

impl<'data, I: IntoParallelRefIterator<'data>> IntoLoggedParallelRefIterator<'data> for I {}

/// This trait extends `IntoParallelIterator`s by providing logging facilities.
pub trait IntoLoggedParallelIterator: IntoParallelIterator + Sized {
    /// Get a parallel logging iterator.
    fn into_par_iter(self) -> Logged<Self::Iter> {
        Logged::new(IntoParallelIterator::into_par_iter(self))
    }
}

impl<I: IntoParallelIterator> IntoLoggedParallelIterator for I {}

pub use rayon_algorithms::slice::ParallelSliceMut;
