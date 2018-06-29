//! `LoggedPool` structure for logging raw tasks events.

use rayon::{join, join_context, prelude::*, FnContext, ThreadPool};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use storage::Storage;
use time::precise_time_ns;

use {RayonEvent, RunLog};

/// ThreadPool for fast and thread safe logging of execution times of tasks.
pub struct LoggedPool {
    /// One vector of events for each thread.
    pub(crate) tasks_logs: Vec<Storage>,
    /// We use an atomic usize to generate unique ids for tasks.
    next_task_id: AtomicUsize,
    /// We use an atomic usize to generate unique ids for iterators.
    next_iterator_id: AtomicUsize,
    /// We need to know the thread pool to figure out thread indices.
    pool: ThreadPool,
    /// When are we created (to shift all recorded times)
    pub(crate) start: u64,
}

unsafe impl Sync for LoggedPool {}

impl LoggedPool {
    /// Create a new events logging structure.
    pub(crate) fn new(pool: ThreadPool) -> Self {
        let n_threads = pool.current_num_threads();
        // warm up the pool immediately
        let m: i32 = pool.install(|| (0..5_000_000).into_par_iter().max().unwrap());
        if m != 4_999_999 {
            panic!("warm up failed")
        }
        LoggedPool {
            tasks_logs: (0..n_threads).map(|_| Storage::new()).collect(),
            next_task_id: ATOMIC_USIZE_INIT,
            next_iterator_id: ATOMIC_USIZE_INIT,
            pool,
            start: precise_time_ns(),
        }
    }
    /// Tag currently active task with a type and amount of work.
    pub fn log_work(&self, work_type: usize, work_amount: usize) {
        self.log(RayonEvent::Work(work_type, work_amount));
    }

    /// Execute a logging join_context.
    pub fn join_context<A, B, RA, RB>(&self, oper_a: A, oper_b: B) -> (RA, RB)
    where
        A: FnOnce(FnContext) -> RA + Send,
        B: FnOnce(FnContext) -> RB + Send,
        RA: Send,
        RB: Send,
    {
        let id_a = self.next_task_id();
        let ca = |c| {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns()));
            let result = oper_a(c);
            self.log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };

        let id_b = self.next_task_id();
        let cb = |c| {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns()));
            let result = oper_b(c);
            self.log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };

        let id_c = self.next_task_id();
        self.log(RayonEvent::Join(id_a, id_b, id_c));
        self.log(RayonEvent::TaskEnd(precise_time_ns()));
        let r = join_context(ca, cb);
        self.log(RayonEvent::TaskStart(id_c, precise_time_ns()));
        r
    }

    /// Erase all logs and resets all counters to 0.
    fn reset(&self) {
        for log in &self.tasks_logs {
            log.clear();
        }
        self.next_task_id.store(0, Ordering::SeqCst);
        self.next_iterator_id.store(0, Ordering::SeqCst);
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    /// After running, we post-process the logs and return a `RunLog` together with the processed data.
    pub fn install<OP, R>(&self, op: OP) -> (R, RunLog)
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        let id = self.next_task_id();
        let c = || {
            self.log(RayonEvent::TaskStart(id, precise_time_ns()));
            let result = op();
            self.log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };
        let r = self.pool.install(c);
        let log = RunLog::new(&self);
        self.reset();
        (r, log)
    }

    /// Execute a logging join.
    pub fn join<A, B, RA, RB>(&self, oper_a: A, oper_b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        let id_a = self.next_task_id();
        let ca = || {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns()));
            let result = oper_a();
            self.log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };

        let id_b = self.next_task_id();
        let cb = || {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns()));
            let result = oper_b();
            self.log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };

        let id_c = self.next_task_id();
        self.log(RayonEvent::Join(id_a, id_b, id_c));
        self.log(RayonEvent::TaskEnd(precise_time_ns()));
        let r = join(ca, cb);
        self.log(RayonEvent::TaskStart(id_c, precise_time_ns()));
        r
    }

    /// Return id for next task (updates counter).
    pub(crate) fn next_task_id(&self) -> usize {
        self.next_task_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Return id for next iterator (updates counter).
    pub(crate) fn next_iterator_id(&self) -> usize {
        self.next_iterator_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Add given event to logs of given thread.
    pub(crate) fn log(&self, event: RayonEvent) {
        if let Some(thread_id) = self.pool.current_thread_index() {
            self.tasks_logs[thread_id].push(event)
        }
    }
}
