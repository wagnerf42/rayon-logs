//! `LoggedPool` structure for logging raw tasks events.

use fork_join_graph::compute_speeds;
use rayon;
use rayon::FnContext;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::{Arc, Mutex};
use storage::Storage;
use time::precise_time_ns;
use TaskId;
use {fill_svg_file, visualisation};
use {svg::histogram, RayonEvent, RunLog, TaskLog};
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

thread_local!(pub(crate) static LOGS: RefCell<Arc<Storage>> = RefCell::new(Arc::new(Storage::new())));

/// Add given event to logs of current thread.
pub(crate) fn log(event: RayonEvent) {
    LOGS.with(|l| l.borrow().push(event))
}

/// Launch a sequential task with tagged work.
/// We expect `op` to be sequential.
pub fn sequential_task<OP, R>(work_type: usize, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    let sequential_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    // log child's work and dependencies.
    log(RayonEvent::SequentialTask(
        sequential_task_id,
        continuation_task_id,
        work_type,
        work_amount,
    ));
    // end current task
    log(RayonEvent::TaskEnd(precise_time_ns()));
    // execute full sequential task
    log(RayonEvent::TaskStart(sequential_task_id, precise_time_ns()));
    let r = op();
    log(RayonEvent::TaskEnd(precise_time_ns()));

    // start continuation task
    log(RayonEvent::TaskStart(
        continuation_task_id,
        precise_time_ns(),
    ));
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
    let id_a = next_task_id();
    let ca = |c| {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a(c);
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_b = next_task_id();
    let cb = |c| {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b(c);
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_c = next_task_id();
    log(RayonEvent::Join(id_a, id_b, id_c));
    log(RayonEvent::TaskEnd(precise_time_ns()));
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
    let id_a = next_task_id();
    let ca = || {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a();
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_b = next_task_id();
    let cb = || {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b();
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_c = next_task_id();
    log(RayonEvent::Join(id_a, id_b, id_c));
    log(RayonEvent::TaskEnd(precise_time_ns()));
    let r = rayon::join(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

/// We wrap rayon's pool into our own struct to overload the install method.
pub struct ThreadPool {
    pub(crate) logs: Arc<Mutex<Vec<Arc<Storage>>>>,
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
    pub fn install<OP, R>(&self, op: OP) -> (R, RunLog)
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

    /// We automatically benchmark and compare two algorithms.
    /// We run 300 tests for each algorithm.
    /// Each time we prepare the experiment and launch both algorithms.
    /// We display some statistics on running times and compare both average and best
    /// runs.
    /// Output is saved on given html file.
    pub fn compare<A, B, P: AsRef<Path>>(
        &self,
        label1: &str,
        label2: &str,
        algo1: A,
        algo2: B,
        filename: P,
    ) -> Result<(), Error>
    where
        A: Fn() + Send + Sync,
        B: Fn() + Send + Sync,
    {
        let tests_number = 100;
        let mut logs = vec![Vec::new(), Vec::new()];
        for _ in 0..tests_number {
            logs[0].push(self.install(&algo1).1);
            logs[1].push(self.install(&algo2).1);
        }
        logs[0].sort_unstable_by_key(|l| l.duration);
        logs[1].sort_unstable_by_key(|l| l.duration);

        let mut html_file = File::create(filename)?;

        write!(html_file, "<!DOCTYPE html>")?;
        write!(html_file, "<html><body><center>")?;
        write!(html_file, "<H1>Comparing {} and {}</H1>", label1, label2)?;

        write!(
            html_file,
            "<H2>Distribution of execution times over {} runs (red is {}, blue is {})</H2>",
            tests_number, label1, label2,
        )?;
        histogram(&mut html_file, &logs, 30)?;

        write!(html_file, "<H2>Comparing median runs</H2>")?;
        let median_index = tests_number / 2;
        let mut speeds: HashMap<usize, f64> = HashMap::new();
        for log in &logs {
            compute_speeds(&log[median_index].tasks_logs, &mut speeds);
        }
        for log in &logs {
            let scene = visualisation(&log[median_index], Some(&speeds));
            fill_svg_file(&scene, &mut html_file)?;
            writeln!(html_file, "<p>")?;
        }

        write!(html_file, "<H2>Comparing best runs</H2>")?;
        speeds.clear();
        for log in &logs {
            compute_speeds(&log[0].tasks_logs, &mut speeds);
        }
        for log in &logs {
            let scene = visualisation(&log[0], Some(&speeds));
            fill_svg_file(&scene, &mut html_file)?;
            writeln!(html_file, "<p>")?;
        }

        write!(html_file, "</body></html>")?;
        Ok(())
    }
}
