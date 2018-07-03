//! We define here all traits enhancing parallel iterators.
pub use rayon::prelude::FromParallelIterator;
pub use rayon::prelude::IndexedParallelIterator;
pub use rayon::prelude::IntoParallelIterator;
pub use rayon::prelude::IntoParallelRefIterator;
pub use rayon::prelude::IntoParallelRefMutIterator;
pub use rayon::prelude::ParallelExtend;
pub use rayon::prelude::ParallelIterator;
pub use rayon::prelude::ParallelSlice;
pub use rayon::prelude::ParallelSliceMut;
pub use rayon::prelude::ParallelString;

use Logged;

/// This trait extends `ParallelItertor`s by providing logging facilities.
pub trait LoggedParallelIterator: ParallelIterator {
    /// Log all thread activities in the provided LoggedPool.
    fn log(self) -> Logged<Self> {
        Logged::new(self)
    }
}

impl<I: ParallelIterator> LoggedParallelIterator for I {}
