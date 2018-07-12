//! Parallel iterator types for [slices][std::slice]
//!
//! You will rarely need to interact with this module directly unless you need
//! to name one of the iterator types.
//!
//! [std::slice]: https://doc.rust-lang.org/stable/std/slice/

mod mergesort;
use self::mergesort::par_mergesort;
use rayon::prelude::ParallelSliceMut as RayonParallelSliceMut;
use std::cmp::Ordering;

/// Parallel extensions for mutable slices.
pub trait ParallelSliceMut<T: Send>: RayonParallelSliceMut<T> {
    /// Sorts the slice in parallel.
    ///
    /// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`par_sort_unstable`](#method.par_sort_unstable).
    ///
    /// # Current implementation
    ///
    /// The current algorithm is an adaptive merge sort inspired by
    /// [timsort](https://en.wikipedia.org/wiki/Timsort).
    /// It is designed to be very fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another.
    ///
    /// Also, it allocates temporary storage the same size as `self`, but for very short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// In order to sort the slice in parallel, the slice is first divided into smaller chunks and
    /// all chunks are sorted in parallel. Then, adjacent chunks that together form non-descending
    /// or descending runs are concatenated. Finally, the remaining chunks are merged together using
    /// parallel subdivision of chunks and parallel merge operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = [-5, 4, 1, -3, 2];
    ///
    /// v.par_sort();
    /// assert_eq!(v, [-5, -3, 1, 2, 4]);
    /// ```
    fn par_sort(&mut self)
    where
        T: Ord,
    {
        par_mergesort(self.as_parallel_slice_mut(), |a, b| a.lt(b));
    }

    /// Sorts the slice in parallel with a comparator function.
    ///
    /// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`par_sort_unstable_by`](#method.par_sort_unstable_by).
    ///
    /// # Current implementation
    ///
    /// The current algorithm is an adaptive merge sort inspired by
    /// [timsort](https://en.wikipedia.org/wiki/Timsort).
    /// It is designed to be very fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another.
    ///
    /// Also, it allocates temporary storage the same size as `self`, but for very short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// In order to sort the slice in parallel, the slice is first divided into smaller chunks and
    /// all chunks are sorted in parallel. Then, adjacent chunks that together form non-descending
    /// or descending runs are concatenated. Finally, the remaining chunks are merged together using
    /// parallel subdivision of chunks and parallel merge operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = [5, 4, 1, 3, 2];
    /// v.par_sort_by(|a, b| a.cmp(b));
    /// assert_eq!(v, [1, 2, 3, 4, 5]);
    ///
    /// // reverse sorting
    /// v.par_sort_by(|a, b| b.cmp(a));
    /// assert_eq!(v, [5, 4, 3, 2, 1]);
    /// ```
    fn par_sort_by<F>(&mut self, compare: F)
    where
        F: Fn(&T, &T) -> Ordering + Sync,
    {
        par_mergesort(self.as_parallel_slice_mut(), |a, b| {
            compare(a, b) == Ordering::Less
        });
    }

    /// Sorts the slice in parallel with a key extraction function.
    ///
    /// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`par_sort_unstable_by_key`](#method.par_sort_unstable_by_key).
    ///
    /// # Current implementation
    ///
    /// The current algorithm is an adaptive merge sort inspired by
    /// [timsort](https://en.wikipedia.org/wiki/Timsort).
    /// It is designed to be very fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another.
    ///
    /// Also, it allocates temporary storage the same size as `self`, but for very short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// In order to sort the slice in parallel, the slice is first divided into smaller chunks and
    /// all chunks are sorted in parallel. Then, adjacent chunks that together form non-descending
    /// or descending runs are concatenated. Finally, the remaining chunks are merged together using
    /// parallel subdivision of chunks and parallel merge operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = [-5i32, 4, 1, -3, 2];
    ///
    /// v.par_sort_by_key(|k| k.abs());
    /// assert_eq!(v, [1, 2, -3, 4, -5]);
    /// ```
    fn par_sort_by_key<B, F>(&mut self, f: F)
    where
        B: Ord,
        F: Fn(&T) -> B + Sync,
    {
        par_mergesort(self.as_parallel_slice_mut(), |a, b| f(a).lt(&f(b)));
    }
}

impl<T: Send> ParallelSliceMut<T> for [T] {}
