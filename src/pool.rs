//! `LoggedPool` structure for logging tasks activities.

use rayon::{join, join_context, FnContext, ThreadPool};
use serde_json;
use std::fs::File;
use std::io;
use std::ops::Drop;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use storage::Storage;
use time::precise_time_ns;

use {RayonEvent, TaskId, TaskLog};

/// ThreadPool for fast and thread safe logging of execution times of tasks.
pub struct LoggedPool {
    /// One vector of events for each thread.
    tasks_logs: Vec<Storage>,
    /// We use an atomic usize to generate unique ids for tasks.
    next_task_id: AtomicUsize,
    /// We use an atomic usize to generate unique ids for iterators.
    next_iterator_id: AtomicUsize,
    /// We need to know the thread pool to figure out thread indices.
    pool: ThreadPool,
    /// If we have a filename here, we automatically save logs on drop.
    logs_filename: Option<String>,
    /// When are we created (to shift all recorded times)
    pub(crate) start: u64,
}

impl Drop for LoggedPool {
    fn drop(&mut self) {
        if let Some(ref filename) = self.logs_filename {
            self.save_logs(filename).expect("saving logs failed");
        }
    }
}

unsafe impl Sync for LoggedPool {}

impl LoggedPool {
    /// Create a new events logging structure.
    pub(crate) fn new(pool: ThreadPool, logs_filename: Option<String>) -> Self {
        let n_threads = pool.current_num_threads();
        LoggedPool {
            tasks_logs: (0..n_threads).map(|_| Storage::new()).collect(),
            next_task_id: ATOMIC_USIZE_INIT,
            next_iterator_id: ATOMIC_USIZE_INIT,
            pool,
            logs_filename,
            start: precise_time_ns(),
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
        let id_a = self.next_task_id();
        let ca = |c| {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns() - self.start));
            let result = oper_a(c);
            self.log(RayonEvent::TaskEnd(id_a, precise_time_ns() - self.start));
            result
        };

        let id_b = self.next_task_id();
        let cb = |c| {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns() - self.start));
            let result = oper_b(c);
            self.log(RayonEvent::TaskEnd(id_b, precise_time_ns() - self.start));
            result
        };

        self.log(RayonEvent::Join(id_a, id_b));

        join_context(ca, cb)
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    pub fn install<OP, R>(&self, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        let id = self.next_task_id();
        let c = || {
            self.log(RayonEvent::TaskStart(id, precise_time_ns() - self.start));
            let result = op();
            self.log(RayonEvent::TaskEnd(id, precise_time_ns() - self.start));
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
        let id_a = self.next_task_id();
        let ca = || {
            self.log(RayonEvent::TaskStart(id_a, precise_time_ns() - self.start));
            let result = oper_a();
            self.log(RayonEvent::TaskEnd(id_a, precise_time_ns() - self.start));
            result
        };

        let id_b = self.next_task_id();
        let cb = || {
            self.log(RayonEvent::TaskStart(id_b, precise_time_ns() - self.start));
            let result = oper_b();
            self.log(RayonEvent::TaskEnd(id_b, precise_time_ns() - self.start));
            result
        };

        self.log(RayonEvent::Join(id_a, id_b));

        join(ca, cb)
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

        let iterators_number = self.next_iterator_id.load(Ordering::SeqCst);
        let mut iterators_info: Vec<_> = (0..iterators_number).map(|_| Vec::new()).collect();
        let mut iterators_fathers = Vec::new();

        for (thread_id, thread_log) in self.tasks_logs.iter().enumerate() {
            thread_log.logs().fold(
                Vec::new(),
                |mut active_tasks: Vec<TaskId>, event: &RayonEvent| -> Vec<TaskId> {
                    match *event {
                        RayonEvent::Join(a, b) => {
                            if let Some(active_task) = active_tasks.last() {
                                tasks_info[*active_task].children.push(a);
                                tasks_info[*active_task].children.push(b);
                            }
                            active_tasks
                        }
                        RayonEvent::TaskEnd(task, time) => {
                            tasks_info[task].end_time = time;
                            active_tasks.pop();
                            active_tasks
                        }
                        RayonEvent::TaskStart(task, time) => {
                            tasks_info[task].thread_id = thread_id;
                            tasks_info[task].start_time = time;
                            active_tasks.push(task);
                            active_tasks
                        }
                        RayonEvent::IteratorTask(task, iterator, part) => {
                            let start = if let Some((start, _)) = part {
                                start
                            } else {
                                0
                            };
                            iterators_info[iterator].push((task, start));
                            active_tasks
                        }
                        RayonEvent::IteratorStart(iterator) => {
                            if let Some(active_task) = active_tasks.last() {
                                iterators_fathers.push((iterator, *active_task));
                            }
                            active_tasks
                        }
                    }
                },
            );
        }

        // now parse iterator info to link iterator tasks to graph
        for (iterator, father) in &iterators_fathers {
            let mut children = &mut iterators_info[*iterator];
            children.sort_unstable_by_key(|(_, start)| *start);
            tasks_info[*father]
                .children
                .extend(children.iter().map(|(task, _)| task));
        }

        serde_json::to_writer(file, &tasks_info).expect("failed serializing");
        Ok(())
    }
}
