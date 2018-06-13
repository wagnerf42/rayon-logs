//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
#![feature(unboxed_closures, fn_traits)]
extern crate rayon;

extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

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
    children: Option<(TaskId, TaskId)>,
}

/// Structs for fast and thread safe logging of execution times of tasks.
/// Each registry contains one.
pub struct Logger<'a> {
    /// One vector of events for each thread.
    tasks_logs: Vec<UnsafeCell<Vec<RayonEvent>>>,
    /// We use an atomic usize to generate unique ids for tasks.
    next_task_id: AtomicUsize,
    /// We need to know the thread pool to figure out thread indices.
    pool: &'a ThreadPool,
}

unsafe impl<'a> Sync for Logger<'a> {}

const MAX_LOGGED_TASKS: usize = 10_000;

/// Encapsulate some closure into another one which will log execution times.
pub struct LoggingClosure<'a: 'b, 'b, RA: Send, A: FnOnce(FnContext) -> RA + Send> {
    /// Real code to execute
    pub real_closure: A,
    /// Our unique task id
    pub id: TaskId,
    /// Where do we store logs ?
    pub logger: &'b Logger<'a>,
}

impl<'a> Logger<'a> {
    /// Create a new events logging structure.
    pub fn new(pool: &'a ThreadPool) -> Self {
        let n_threads = pool.current_num_threads();
        Logger {
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
        let ca = self.logging_closure(oper_a);
        let cb = self.logging_closure(oper_b);
        self.log(RayonEvent::Join(ca.id, cb.id));

        rayon::join_context(ca, cb)
    }

    /// Execute a logging join.
    pub fn join<A, B, RA, RB>(&self, oper_a: A, oper_b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        let ca = self.logging_closure(|_| oper_a());
        let cb = self.logging_closure(|_| oper_b());
        self.log(RayonEvent::Join(ca.id, cb.id));

        rayon::join_context(ca, cb)
    }

    /// Create a new closure which will log record and log execution times on execution.
    /// It gets a unique ID.
    pub fn logging_closure<RA, A>(&self, oper: A) -> LoggingClosure<RA, A>
    where
        A: FnOnce(FnContext) -> RA + Send,
        RA: Send,
    {
        LoggingClosure {
            real_closure: oper,
            id: self.next_task_id.fetch_add(1, Ordering::SeqCst),
            logger: &self,
        }
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
        let mut buffer = File::create(path)?;

        let tasks_number = self.next_task_id.load(Ordering::SeqCst);
        let mut tasks_info: Vec<TaskLog> = Vec::with_capacity(tasks_number);
        unsafe {
            tasks_info.set_len(tasks_number);
        }
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
                                tasks_info[*active_task].children = Some((a, b));
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

        buffer.write_fmt(format_args!(
            "{}",
            serde_json::to_string(&tasks_info).unwrap()
        ))
    }
}

impl<'a, 'b, RA: Send, A: FnOnce(FnContext) -> RA + Send> FnOnce<(FnContext,)>
    for LoggingClosure<'a, 'b, RA, A>
{
    type Output = RA;
    extern "rust-call" fn call_once(self, context: (FnContext,)) -> Self::Output {
        {
            self.logger
                .log(RayonEvent::TaskStart(self.id, precise_time_ns()));
            let result = (self.real_closure)(context.0);
            self.logger
                .log(RayonEvent::TaskEnd(self.id, precise_time_ns()));
            result
        }
    }
}
