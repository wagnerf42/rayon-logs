//! All code which will move inside rayon.
#![macro_use]

use crate::common::raw_events::{RawEvent, TaskId, TimeStamp};
use rayon::FnContext;
use std::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
lazy_static! {
    static ref START_TIME: std::time::Instant = std::time::Instant::now();
}

/// Return number of nano seconds since start.
pub(crate) fn now() -> TimeStamp {
    START_TIME.elapsed().as_nanos() as TimeStamp
}

/// Add given event to logs of current thread.
pub(crate) fn log(event: RawEvent<&'static str>) {
    recorder::THREAD_LOGS.with(|l| l.push(event))
}

/// Logs several events at once (with decreased cost).
macro_rules! logs {
    ($($x:expr ), +) => {
        $crate::rayon::recorder::THREAD_LOGS.with(|l| {
            $(
                l.push($x);
              )*
        })
    }
}

/// We use an atomic usize to generate unique ids for tasks.
/// We start at 1 since initial task (0) is created manually.
pub(crate) static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

/// get an id for a new task and increment global tasks counter.
pub fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// Identical to `join`, except that the closures have a parameter
/// that provides context for the way the closure has been called,
/// especially indicating whether they're executing on a different
/// thread than where `join_context` was called.  This will occur if
/// the second job is stolen by a different thread, or if
/// `join_context` was called from outside the thread pool to begin
/// with.
pub fn join_context<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce(FnContext) -> RA + Send,
    B: FnOnce(FnContext) -> RB + Send,
    RA: Send,
    RB: Send,
{
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = |c| {
        log(RawEvent::TaskStart(id_a, now()));
        let result = oper_a(c);
        logs!(RawEvent::Child(id_c), RawEvent::TaskEnd(now()));
        result
    };

    let id_b = next_task_id();
    let cb = |c| {
        log(RawEvent::TaskStart(id_b, now()));
        let result = oper_b(c);
        logs!(RawEvent::Child(id_c), RawEvent::TaskEnd(now()));
        result
    };

    logs!(
        RawEvent::Child(id_a),
        RawEvent::Child(id_b),
        RawEvent::TaskEnd(now())
    );
    let r = rayon::join_context(ca, cb);
    log(RawEvent::TaskStart(id_c, now()));
    r
}

/// Takes two closures and *potentially* runs them in parallel. It
/// returns a pair of the results from those closures.
///
/// Conceptually, calling `join()` is similar to spawning two threads,
/// one executing each of the two closures. However, the
/// implementation is quite different and incurs very low
/// overhead. The underlying technique is called "work stealing": the
/// Rayon runtime uses a fixed pool of worker threads and attempts to
/// only execute code in parallel when there are idle CPUs to handle
/// it.
///
/// When `join` is called from outside the thread pool, the calling
/// thread will block while the closures execute in the pool.  When
/// `join` is called within the pool, the calling thread still actively
/// participates in the thread pool. It will begin by executing closure
/// A (on the current thread). While it is doing that, it will advertise
/// closure B as being available for other threads to execute. Once closure A
/// has completed, the current thread will try to execute closure B;
/// if however closure B has been stolen, then it will look for other work
/// while waiting for the thief to fully execute closure B. (This is the
/// typical work-stealing strategy).
///
/// # Examples
///
/// This example uses join to perform a quick-sort (note this is not a
/// particularly optimized implementation: if you **actually** want to
/// sort for real, you should prefer [the `par_sort` method] offered
/// by Rayon).
///
/// [the `par_sort` method]: ../rayon/slice/trait.ParallelSliceMut.html#method.par_sort
///
/// ```rust
/// let mut v = vec![5, 1, 8, 22, 0, 44];
/// quick_sort(&mut v);
/// assert_eq!(v, vec![0, 1, 5, 8, 22, 44]);
///
/// fn quick_sort<T:PartialOrd+Send>(v: &mut [T]) {
///    if v.len() > 1 {
///        let mid = partition(v);
///        let (lo, hi) = v.split_at_mut(mid);
///        rayon::join(|| quick_sort(lo),
///                    || quick_sort(hi));
///    }
/// }
///
/// // Partition rearranges all items `<=` to the pivot
/// // item (arbitrary selected to be the last item in the slice)
/// // to the first half of the slice. It then returns the
/// // "dividing point" where the pivot is placed.
/// fn partition<T:PartialOrd+Send>(v: &mut [T]) -> usize {
///     let pivot = v.len() - 1;
///     let mut i = 0;
///     for j in 0..pivot {
///         if v[j] <= v[pivot] {
///             v.swap(i, j);
///             i += 1;
///         }
///     }
///     v.swap(i, pivot);
///     i
/// }
/// ```
///
/// # Warning about blocking I/O
///
/// The assumption is that the closures given to `join()` are
/// CPU-bound tasks that do not perform I/O or other blocking
/// operations. If you do perform I/O, and that I/O should block
/// (e.g., waiting for a network request), the overall performance may
/// be poor.  Moreover, if you cause one closure to be blocked waiting
/// on another (for example, using a channel), that could lead to a
/// deadlock.
///
/// # Panics
///
/// No matter what happens, both closures will always be executed.  If
/// a single closure panics, whether it be the first or second
/// closure, that panic will be propagated and hence `join()` will
/// panic with the same panic value. If both closures panic, `join()`
/// will panic with the panic value from the first closure.
pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = || {
        log(RawEvent::TaskStart(id_a, now()));
        let result = oper_a();
        logs!(RawEvent::Child(id_c), RawEvent::TaskEnd(now()));
        result
    };

    let id_b = next_task_id();
    let cb = || {
        log(RawEvent::TaskStart(id_b, now()));
        let result = oper_b();
        logs!(RawEvent::Child(id_c), RawEvent::TaskEnd(now()));
        result
    };

    logs!(
        RawEvent::Child(id_a),
        RawEvent::Child(id_b),
        RawEvent::TaskEnd(now())
    );
    let r = rayon::join(ca, cb);
    log(RawEvent::TaskStart(id_c, now()));
    r
}

pub(crate) mod list;
pub(crate) mod recorder;
pub(crate) mod scope;
pub(crate) mod storage;
pub(crate) mod subgraphs;
