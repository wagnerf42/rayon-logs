//! `LoggedPool` structure for logging raw tasks events.
#![macro_use]

use crate::raw_events::{RayonEvent, TaskId};
use crate::storage::Storage;
use crate::Comparator;
use crate::RunLog;
use crate::{scope, Scope};
use rayon;
use rayon::FnContext;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::{Arc, Mutex};
use time::precise_time_ns;

/// We use an atomic usize to generate unique ids for tasks.
pub(crate) static NEXT_TASK_ID: AtomicUsize = ATOMIC_USIZE_INIT;
/// We use an atomic usize to generate unique ids for iterators.
pub(crate) static NEXT_ITERATOR_ID: AtomicUsize = ATOMIC_USIZE_INIT;

/// get an id for a new task and increment global tasks counter.
pub fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// get an id for a new iterator and increment global iterators counter.
pub fn next_iterator_id() -> usize {
    NEXT_ITERATOR_ID.fetch_add(1, Ordering::SeqCst)
}

thread_local!(pub(crate) static LOGS: RefCell<Arc<Storage<RayonEvent>>> = RefCell::new(Arc::new(Storage::new())));

/// Add given event to logs of current thread.
pub(crate) fn log(event: RayonEvent) {
    LOGS.with(|l| l.borrow().push(event))
}

/// Logs several events at once (with decreased cost).
macro_rules! logs {
    ($($x:expr ), +) => {
        $crate::pool::LOGS.with(|l| {let thread_logs = l.borrow();
            $(
                thread_logs.push($x);
                )*
        })
    }
}

/// Launch a sequential task with tagged work.
pub fn sequential_task<OP, R>(work_type: &'static str, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    make_subgraph(work_type, work_amount, op)
}

/// We tag all the tasks that op makes as one subgraph. Useful for speed and time computation, and
/// will eventually be added to the SVG for display as well.
/// Svg display is for now available only if `op` is sequential.
pub fn make_subgraph<OP, R>(work_type: &'static str, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    let subgraph_start_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    logs!(
        // log child's work and dependencies.
        RayonEvent::Child(subgraph_start_task_id),
        // end current task
        RayonEvent::TaskEnd(precise_time_ns()),
        // execute full sequential task
        RayonEvent::TaskStart(subgraph_start_task_id, precise_time_ns()),
        RayonEvent::SubgraphStart(work_type, work_amount)
    );
    let r = op();
    logs!(
        RayonEvent::SubgraphEnd(work_type),
        RayonEvent::Child(continuation_task_id),
        RayonEvent::TaskEnd(precise_time_ns()),
        // start continuation task
        RayonEvent::TaskStart(continuation_task_id, precise_time_ns(),)
    );
    r
}

/// Execute a logging join_context.
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
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a(c);
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    let id_b = next_task_id();
    let cb = |c| {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b(c);
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    logs!(
        RayonEvent::Child(id_a),
        RayonEvent::Child(id_b),
        RayonEvent::TaskEnd(precise_time_ns())
    );
    let r = rayon::join_context(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

/// Execute a logging join.
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
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a();
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    let id_b = next_task_id();
    let cb = || {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b();
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    logs!(
        RayonEvent::Child(id_a),
        RayonEvent::Child(id_b),
        RayonEvent::TaskEnd(precise_time_ns())
    );
    let r = rayon::join(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

// small global counter to increment file names
static INSTALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// We wrap rayon's pool into our own struct to overload the install method.
pub struct ThreadPool {
    pub(crate) logs: Arc<Mutex<Vec<Arc<Storage<RayonEvent>>>>>,
    pub(crate) pool: rayon::ThreadPool,
}

impl ThreadPool {
    /// Reset all logs and counters to initial condition.
    fn reset(&self) {
        NEXT_TASK_ID.store(0, Ordering::SeqCst);
        NEXT_ITERATOR_ID.store(0, Ordering::SeqCst);
        let logs = &*self.logs.lock().unwrap(); // oh yeah baby
        for log in logs {
            log.clear();
        }
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    /// After running, we post-process the logs and return a `RunLog` together with the closure's
    /// result.
    pub fn logging_install<OP, R>(&self, op: OP) -> (R, RunLog)
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        self.reset();
        let id = next_task_id();
        let c = || {
            log(RayonEvent::TaskStart(id, precise_time_ns()));
            let result = op();
            log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };
        let start = precise_time_ns();
        let r = self.pool.install(c);
        let log = RunLog::new(
            NEXT_TASK_ID.load(Ordering::Relaxed),
            NEXT_ITERATOR_ID.load(Ordering::Relaxed),
            &*self.logs.lock().unwrap(),
            start,
        );
        (r, log)
    }

    /// Creates a scope that executes within this thread-pool.
    /// Equivalent to `self.install(|| scope(...))`.
    ///
    /// See also: [the `scope()` function][scope].
    ///
    /// [scope]: fn.scope.html
    pub fn scope<'scope, OP, R>(&self, op: OP) -> R
    where
        OP: for<'s> FnOnce(&'s Scope<'scope>) -> R + 'scope + Send,
        R: Send,
    {
        self.install(|| scope(op))
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    /// After running, we save a json file with filename being an incremental counter.
    pub fn install<OP, R>(&self, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        let (r, log) = self.logging_install(op);
        log.save(format!(
            "log_{}.json",
            INSTALL_COUNT.fetch_add(1, Ordering::SeqCst)
        ))
        .expect("saving json failed");
        r
    }

    ///This function simply returns a comparator that allows us to add algorithms for comparison.
    pub fn compare(&self) -> Comparator {
        Comparator::new(self)
    }
}
