//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
extern crate rayon;

extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod iterator;

use rayon::{FnContext, ThreadPool};
///! Small submodule for performance related logs.
//use registry::WorkerThread;
use std::cell::UnsafeCell;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use time::precise_time_ns;

type TaskId = usize;
type TimeStamp = u64;

/// All types of events we can log.
#[derive(Debug, Serialize, Deserialize)]
enum RayonEvent {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// A task ends.
    TaskEnd(TaskId, TimeStamp),
    /// We create two tasks with join (contains dependencies information).
    Join(TaskId, TaskId),
}

/// The final information produced for log viewers.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskLog {
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
}

/// ThreadPool for fast and thread safe logging of execution times of tasks.
pub struct LoggedPool<'a> {
    /// One vector of events for each thread.
    tasks_logs: Vec<UnsafeCell<Vec<RayonEvent>>>,
    /// We use an atomic usize to generate unique ids for tasks.
    next_task_id: AtomicUsize,
    /// We need to know the thread pool to figure out thread indices.
    pool: &'a ThreadPool,
}

unsafe impl<'a> Sync for LoggedPool<'a> {}

const MAX_LOGGED_TASKS: usize = 10_000;

impl<'a> LoggedPool<'a> {
    /// Create a new events logging structure.
    pub fn new(pool: &'a ThreadPool) -> Self {
        let n_threads = pool.current_num_threads();
        LoggedPool {
            tasks_logs: (0..n_threads)
                .map(|_| UnsafeCell::new(Vec::with_capacity(MAX_LOGGED_TASKS)))
                .collect(),
            next_task_id: ATOMIC_USIZE_INIT,
            pool,
        }
    }
    /// Execute a logging join_context.
    pub fn join_context<A, B, RA, RB>(&self, oper_a: A, oper_b: B) -> (RA, RB)
    where
        A: FnOnce(FnContext) -> RA + Send,
        B: FnOnce(FnContext) -> RB + Send,
        RA: Send,
        RB: Send,
    {
        let id_a = self.next_id();
        let ca = |c| {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns()));
            let result = oper_a(c);
            self.log(RayonEvent::TaskEnd(id_a, precise_time_ns()));
            result
        };

        let id_b = self.next_id();
        let cb = |c| {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns()));
            let result = oper_b(c);
            self.log(RayonEvent::TaskEnd(id_b, precise_time_ns()));
            result
        };

        self.log(RayonEvent::Join(id_a, id_b));

        rayon::join_context(ca, cb)
    }

    pub fn install<OP, R>(&self, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        let id = self.next_id();
        let c = || {
            self.log(RayonEvent::TaskStart(id, precise_time_ns()));
            let result = op();
            self.log(RayonEvent::TaskEnd(id, precise_time_ns()));
            result
        };
        self.pool.install(c)
    }

    /// Execute a logging join.
    pub fn join<A, B, RA, RB>(&self, oper_a: A, oper_b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        let id_a = self.next_id();
        let ca = || {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns()));
            let result = oper_a();
            self.log(RayonEvent::TaskEnd(id_a, precise_time_ns()));
            result
        };

        let id_b = self.next_id();
        let cb = || {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns()));
            let result = oper_b();
            self.log(RayonEvent::TaskEnd(id_b, precise_time_ns()));
            result
        };

        self.log(RayonEvent::Join(id_a, id_b));

        rayon::join(ca, cb)
    }

    /// Return id for next task (updates counter).
    fn next_id(&self) -> usize {
        self.next_task_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Add given event to logs of given thread.
    fn log(&self, event: RayonEvent) {
        if let Some(thread_id) = self.pool.current_thread_index() {
            unsafe { self.tasks_logs[thread_id].get().as_mut() }
                .unwrap()
                .push(event)
        }
    }

    /// Save log file of currently recorded tasks logs.
    pub fn save_logs<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let file = File::create(path)?;

        let tasks_number = self.next_task_id.load(Ordering::SeqCst);
        let mut tasks_info: Vec<_> = (0..tasks_number)
            .map(|_| TaskLog {
                start_time: 0, // will be filled later
                end_time: 0,
                thread_id: 0,
                children: Vec::new(),
            })
            .collect();

        // get min time
        let start_time = self
            .tasks_logs
            .iter()
            .filter_map(|l| {
                unsafe { l.get().as_ref() }
                    .unwrap()
                    .iter()
                    .filter_map(|e| match e {
                        RayonEvent::TaskStart(_, time) => Some(time),
                        _ => None,
                    })
                    .next()
            })
            .min()
            .unwrap();

        for (thread_id, thread_log) in self.tasks_logs.iter().enumerate() {
            unsafe { thread_log.get().as_ref() }.unwrap().iter().fold(
                Vec::new(),
                |mut active_tasks: Vec<TaskId>, event: &RayonEvent| -> Vec<TaskId> {
                    match event {
                        &RayonEvent::Join(a, b) => {
                            if let Some(active_task) = active_tasks.last() {
                                tasks_info[*active_task].children.push(a);
                                tasks_info[*active_task].children.push(b);
                            }
                            active_tasks
                        }
                        &RayonEvent::TaskEnd(task, time) => {
                            tasks_info[task].end_time = time - start_time;
                            active_tasks.pop();
                            active_tasks
                        }
                        &RayonEvent::TaskStart(task, time) => {
                            tasks_info[task].thread_id = thread_id;
                            tasks_info[task].start_time = time - start_time;
                            active_tasks.push(task);
                            active_tasks
                        }
                    }
                },
            );
        }
        serde_json::to_writer(file, &tasks_info).expect("failed serializing");
        Ok(())
    }
}
