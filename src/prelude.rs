//! We redefine (not all of them yet) rayon traits.
pub use rayon::prelude::ParallelIterator;

use crate::Logged;

/// `IntoParallelRefIterator` implements the conversion to a
/// [`ParallelIterator`], providing shared references to the data.
///
/// This is a parallel version of the `iter()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefIterator<'data>: rayon::prelude::IntoParallelRefIterator<'data> {
    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..100).collect();
    /// assert_eq!(v.par_iter().sum::<i32>(), 100 * 99 / 2);
    ///
    /// // `v.par_iter()` is shorthand for `(&v).into_par_iter()`,
    /// // producing the exact same references.
    /// assert!(v.par_iter().zip(&v)
    ///          .all(|(a, b)| std::ptr::eq(a, b)));
    /// ```
    fn par_iter(&'data self) -> Logged<Self::Iter> {
        Logged::new(rayon::prelude::IntoParallelRefIterator::par_iter(self))
    }
}

impl<'data, I: rayon::prelude::IntoParallelRefIterator<'data>> IntoParallelRefIterator<'data>
    for I
{
}

/// `IntoParallelIterator` implements the conversion to a [`ParallelIterator`].
///
/// By implementing `IntoParallelIterator` for a type, you define how it will
/// transformed into an iterator. This is a parallel version of the standard
/// library's [`std::iter::IntoIterator`] trait.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`std::iter::IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
pub trait IntoParallelIterator: rayon::prelude::IntoParallelIterator + Sized {
    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// println!("counting in parallel:");
    /// (0..100).into_par_iter()
    ///     .for_each(|i| println!("{}", i));
    /// ```
    ///
    /// This conversion is often implicit for arguments to methods like [`zip`].
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..5).into_par_iter().zip(5..10).collect();
    /// assert_eq!(v, [(0, 5), (1, 6), (2, 7), (3, 8), (4, 9)]);
    /// ```
    ///
    /// [`zip`]: trait.IndexedParallelIterator.html#method.zip
    fn into_par_iter(self) -> Logged<Self::Iter> {
        Logged::new(rayon::prelude::IntoParallelIterator::into_par_iter(self))
    }
}

impl<I: rayon::prelude::IntoParallelIterator> IntoParallelIterator for I {}

/// `IntoParallelRefMutIterator` implements the conversion to a
/// [`ParallelIterator`], providing mutable references to the data.
///
/// This is a parallel version of the `iter_mut()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &mut I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefMutIterator<'data>:
    rayon::prelude::IntoParallelRefMutIterator<'data>
{
    /// Creates the parallel iterator from `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = vec![0usize; 5];
    /// v.par_iter_mut().enumerate().for_each(|(i, x)| *x = i);
    /// assert_eq!(v, [0, 1, 2, 3, 4]);
    /// ```
    fn par_iter_mut(&'data mut self) -> Logged<Self::Iter> {
        Logged::new(rayon::prelude::IntoParallelRefMutIterator::par_iter_mut(
            self,
        ))
    }
}
impl<'data, I: rayon::prelude::IntoParallelRefMutIterator<'data>> IntoParallelRefMutIterator<'data>
    for I
{
}

pub use crate::rayon_algorithms::slice::ParallelSliceMut;
// For the subgraph_perf function
#[cfg(feature = "perf")]
pub use perfcnt::linux::HardwareEventType;
