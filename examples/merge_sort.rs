//! you should read only the main function.
//! the merge sort algorithms here are quite complex

extern crate itertools;
extern crate rand;
extern crate rayon_logs;
use rayon_logs::{join, join_context, sequential_task, ThreadPoolBuilder};

use rand::{ChaChaRng, Rng};
use std::fmt::Debug;
use std::iter::repeat;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;

const SORT_SEQUENTIAL_LIMIT: usize = 1000;
const MERGE_SEQUENTIAL_LIMIT: usize = 1000;
const ADAPTIVE_BLOCK_SIZE: usize = 200;

trait Boolean {
    fn value() -> bool;
}
struct True;
struct False;
impl Boolean for True {
    fn value() -> bool {
        true
    }
}
impl Boolean for False {
    fn value() -> bool {
        false
    }
}

/// find subslice without last value in given sorted slice.
fn subslice_without_last_value<T: Eq>(slice: &[T]) -> &[T] {
    match slice.split_last() {
        Some((target, slice)) => {
            let searching_range_start = repeat(())
        .scan(1, |acc, _| {*acc *= 2 ; Some(*acc)}) // iterate on all powers of 2
        .take_while(|&i| i < slice.len())
        .map(|i| slice.len() -i) // go farther and farther from end of slice
        .find(|&i| unsafe {slice.get_unchecked(i) != target})
        .unwrap_or(0);

            let index = slice[searching_range_start..]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                })
                .unwrap_err();
            &slice[0..(searching_range_start + index)]
        }
        None => slice,
    }
}

/// find subslice without first value in given sorted slice.
fn subslice_without_first_value<T: Eq>(slice: &[T]) -> &[T] {
    match slice.first() {
        Some(target) => {
            let searching_range_end = repeat(())
        .scan(1, |acc, _| {*acc *= 2; Some(*acc)}) // iterate on all powers of 2
        .take_while(|&i| i < slice.len())
        .find(|&i| unsafe {slice.get_unchecked(i) != target})
        .unwrap_or_else(||slice.len());

            let index = slice[..searching_range_end]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
                .unwrap_err();
            &slice[index..]
        }
        None => slice,
    }
}

/// Cut sorted slice `slice` around start point, splitting around
/// all values equal to value at start point.
/// cost is O(log(|removed part size|))
fn split_around<T: Eq>(slice: &[T], start: usize) -> (&[T], &[T], &[T]) {
    let low_slice = subslice_without_last_value(&slice[0..(start + 1)]);
    let high_slice = subslice_without_first_value(&slice[start..]);
    let equal_slice = &slice[low_slice.len()..slice.len() - high_slice.len()];
    (low_slice, equal_slice, high_slice)
}

pub trait MergingStrategy {
    fn merge<T: Ord + Copy + Send + Sync + Debug>(left: &[T], right: &[T], output: &mut [T]);
}

struct SequentialMerge;
impl MergingStrategy for SequentialMerge {
    fn merge<T: Ord + Copy + Send + Sync + Debug>(left: &[T], right: &[T], output: &mut [T]) {
        sequential_task(0, output.len(), || {
            partial_manual_merge::<True, True, False, _>(left, right, output, 0)
        });
    }
}

struct ParallelMerge;
impl MergingStrategy for ParallelMerge {
    fn merge<T: Ord + Copy + Send + Sync + Debug>(left: &[T], right: &[T], output: &mut [T]) {
        if left.len() < MERGE_SEQUENTIAL_LIMIT && right.len() < MERGE_SEQUENTIAL_LIMIT {
            partial_manual_merge::<True, True, False, _>(left, right, output, 0);
        } else {
            let ((left1, left2, left3), (right1, right2, right3)) = if left.len() > right.len() {
                merge_split(left, right)
            } else {
                let (split_right, split_left) = merge_split(right, left);
                (split_left, split_right)
            };
            let (output1, output_end) = output.split_at_mut(left1.len() + right1.len());
            let (output2, output3) = output_end.split_at_mut(left2.len() + right2.len());
            let (output2a, output2b) = output2.split_at_mut(left2.len());
            join(
                || {
                    join(
                        || output2a.copy_from_slice(left2),
                        || output2b.copy_from_slice(right2),
                    )
                },
                || {
                    join(
                        || ParallelMerge::merge(left1, right1, output1),
                        || ParallelMerge::merge(left3, right3, output3),
                    )
                },
            );
        }
    }
}

struct AdaptiveMerge;
impl MergingStrategy for AdaptiveMerge {
    fn merge<T: Ord + Copy + Send + Sync + Debug>(left: &[T], right: &[T], output: &mut [T]) {
        let stolen = &AtomicBool::new(false);
        let (tx, rx) = channel();
        join_context(
            move |_| {
                let mut left_done = 0;
                let mut right_done = 0;
                let mut output_done = 0;
                let size = left.len() + right.len();
                //let seq_size: usize = (size as f64).sqrt() as usize;
                let seq_size: usize = ADAPTIVE_BLOCK_SIZE;
                while !stolen.load(Ordering::Relaxed) && output_done < size {
                    if let Some((left_read, right_read, output_written)) = match (
                        left.len() - left_done < seq_size,
                        right.len() - right_done < seq_size,
                    ) {
                        (false, false) => partial_manual_merge::<False, False, True, _>(
                            &left[left_done..],
                            &right[right_done..],
                            &mut output[output_done..],
                            seq_size,
                        ),
                        (true, true) => partial_manual_merge::<True, True, False, _>(
                            &left[left_done..],
                            &right[right_done..],
                            &mut output[output_done..],
                            seq_size,
                        ),
                        (true, false) => partial_manual_merge::<True, False, True, _>(
                            &left[left_done..],
                            &right[right_done..],
                            &mut output[output_done..],
                            seq_size,
                        ),
                        (false, true) => partial_manual_merge::<False, True, True, _>(
                            &left[left_done..],
                            &right[right_done..],
                            &mut output[output_done..],
                            seq_size,
                        ),
                    } {
                        left_done += left_read;
                        right_done += right_read;
                        output_done += output_written;
                    } else {
                        output_done = size;
                    }
                }
                if output_done == size {
                    tx.send(None).expect("sending no work failed");
                } else {
                    let (_, remaining_left) = left.split_at(left_done);
                    let (_, remaining_right) = right.split_at(right_done);
                    let (_, remaining_output) = output.split_at_mut(output_done);
                    if remaining_left.len() < MERGE_SEQUENTIAL_LIMIT
                        && remaining_right.len() < MERGE_SEQUENTIAL_LIMIT
                    {
                        SequentialMerge::merge(remaining_left, remaining_right, remaining_output);
                        tx.send(None).expect("sending no work failed");
                    } else {
                        let ((left1, left2, left3), (right1, right2, right3)) =
                            if remaining_left.len() > remaining_right.len() {
                                merge_split(remaining_left, remaining_right)
                            } else {
                                let (split_right, split_left) =
                                    merge_split(remaining_right, remaining_left);
                                (split_left, split_right)
                            };
                        let (size1, size2) =
                            (left1.len() + right1.len(), left2.len() + right2.len());
                        let (output_start, output3) = remaining_output.split_at_mut(size1 + size2);
                        tx.send(Some((left3, right3, output3)))
                            .expect("sending work failed");
                        let (output1, output2) = output_start.split_at_mut(size1);
                        let (output2a, output2b) = output2.split_at_mut(left2.len());
                        join(
                            || {
                                join(
                                    || output2a.copy_from_slice(left2),
                                    || output2b.copy_from_slice(right2),
                                )
                            },
                            || AdaptiveMerge::merge(left1, right1, output1),
                        );
                    }
                }
            },
            move |c| {
                if c.migrated() {
                    stolen.store(true, Ordering::Relaxed);
                    if let Some((slice_left, slice_right, slice_output)) =
                        rx.recv().expect("receiving prefix failed")
                    {
                        AdaptiveMerge::merge(slice_left, slice_right, slice_output)
                    }
                } else {
                    //assert_eq!(None, rx.recv().expect("receiving prefix failed"));
                }
            },
        );
    }
}

// recursive merge sort, ping pong between two buffers
fn recursive_parallel_merge_sort<T: Ord + Copy + Send + Sync + Debug, M: MergingStrategy>(
    input: &mut [T],
    output: &mut [T],
    recursions: u8,
    sequential: bool,
) {
    if recursions == 0 {
        let mut size = input.len() as f64;
        size *= size.log2();
        sequential_task(1, size as usize, || input.sort());
    } else {
        let midpoint = input.len() / 2;
        let (out1, out2) = output.split_at_mut(midpoint);
        {
            let (in1, in2) = input.split_at_mut(midpoint);
            if sequential {
                recursive_parallel_merge_sort::<T, SequentialMerge>(
                    out1,
                    in1,
                    recursions - 1,
                    true,
                );
                recursive_parallel_merge_sort::<T, SequentialMerge>(
                    out2,
                    in2,
                    recursions - 1,
                    true,
                );
            } else {
                join_context(
                    |_| recursive_parallel_merge_sort::<T, M>(out1, in1, recursions - 1, false),
                    |c| {
                        recursive_parallel_merge_sort::<T, M>(
                            out2,
                            in2,
                            recursions - 1,
                            !c.migrated(),
                        )
                    },
                );
            }
        }
        M::merge(out1, out2, input);
    }
}

/// parallel stable sort (uses one extra array)
pub fn parallel_merge_sort<T: Ord + Copy + Send + Sync + Debug, M: MergingStrategy>(
    slice: &mut [T],
) {
    // we start by computing how many recursion levels we will need.
    // this way we can adjust it by 1 level in order to avoid needing a third buffer.
    let mut recursions =
        ((slice.len() as f64).log(2.0) - (SORT_SEQUENTIAL_LIMIT as f64).log(2.0)).ceil() as i16;
    if recursions % 2 == 1 {
        recursions -= 1;
    }
    if recursions <= 0 {
        slice.sort();
        return;
    }
    let mut buffer = Vec::with_capacity(slice.len());
    unsafe {
        buffer.set_len(slice.len());
    }
    recursive_parallel_merge_sort::<T, M>(slice, &mut buffer, recursions as u8, false);
}

fn partial_manual_merge<
    CheckLeft: Boolean,
    CheckRight: Boolean,
    CheckLimit: Boolean,
    T: Ord + Copy,
>(
    input1: &[T],
    input2: &[T],
    output: &mut [T],
    limit: usize,
) -> Option<(usize, usize, usize)> {
    let mut i1 = 0;
    let mut i2 = 0;
    let mut i_out = 0;
    let (check_left, check_right, check_limit) =
        (CheckLeft::value(), CheckRight::value(), CheckLimit::value());
    if check_limit {
        debug_assert_eq!(true, input1.len() + input2.len() >= limit);
        debug_assert_eq!(true, limit <= output.len());
    } else {
        debug_assert_eq!(input1.len() + input2.len(), output.len());
    }
    if check_left && input1.is_empty() {
        output.copy_from_slice(input2);
        return None;
    } else if check_right && input2.is_empty() {
        output.copy_from_slice(input1);
        return None;
    } else {
        unsafe {
            let mut value1 = input1.get_unchecked(i1);
            let mut value2 = input2.get_unchecked(i2);
            for o in output.iter_mut() {
                if check_limit && i_out >= limit {
                    break;
                }
                if value1.lt(value2) {
                    *o = *value1;
                    i1 += 1;
                    i_out += 1;
                    if check_right && i1 >= input1.len() {
                        break;
                    }
                    value1 = input1.get_unchecked(i1);
                } else {
                    *o = *value2;
                    i2 += 1;
                    i_out += 1;
                    if check_right && i2 >= input2.len() {
                        break;
                    }
                    value2 = input2.get_unchecked(i2);
                }
            }
        }
        if check_right && i2 == input2.len() {
            output[(i1 + i2)..].copy_from_slice(&input1[i1..]);
            return None;
        } else if check_left && i1 == input1.len() {
            output[(i1 + i2)..].copy_from_slice(&input2[i2..]);
            return None;
        }
    }
    Some((i1, i2, i_out))
}

/// split large array at midpoint and small array where needed for merge.
fn merge_split<'a, T: Ord>(
    large: &'a [T],
    small: &'a [T],
) -> ((&'a [T], &'a [T], &'a [T]), (&'a [T], &'a [T], &'a [T])) {
    let middle = large.len() / 2;
    let split_large = split_around(large, middle);
    let split_small = match small.binary_search(&large[middle]) {
        Ok(i) => split_around(small, i),
        Err(i) => {
            let (small1, small3) = small.split_at(i);
            (small1, &small[0..0], small3)
        }
    };
    (split_large, split_small)
}

fn main() {
    let mut ra = ChaChaRng::new_unseeded();

    let mut v: Vec<u32> = (0..100_000).collect();
    let answer = v.clone();
    ra.shuffle(&mut v);

    let p = ThreadPoolBuilder::new().build().expect("builder failed");

    p.compare(
        "merge sort with sequential merge",
        "merge sort with parallel merge",
        || {
            let mut w = v.clone();
            parallel_merge_sort::<u32, SequentialMerge>(&mut w);
            assert_eq!(answer, w);
        },
        || {
            let mut w = v.clone();
            parallel_merge_sort::<u32, ParallelMerge>(&mut w);
            assert_eq!(answer, w);
        },
        "merge_sorts.html",
    ).expect("failed saving comparison results");
}
