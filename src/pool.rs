//! `LoggedPool` structure for logging raw tasks events.

use fork_join_graph::compute_avg_speeds;
use fork_join_graph::compute_speeds;
use itertools::Itertools;
use rayon;
use rayon::FnContext;
use stats::Stats;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::iter::repeat_with;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::{Arc, Mutex};
use storage::Storage;
use time::precise_time_ns;
use {fill_svg_file, visualisation};
use {scope, Scope, TaskId};
use {
    svg::{histogram, HISTOGRAM_COLORS},
    RayonEvent, RunLog,
};

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

/// Add a label and work amount tag to the currently running task.
pub fn tag_task(work_type: &'static str, work_amount: usize) {
    log(RayonEvent::Tag(work_type, work_amount))
}

/// Launch a sequential task with tagged work.
/// We expect `op` to be sequential.
pub fn sequential_task<OP, R>(work_type: &'static str, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    let sequential_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    // log child's work and dependencies.
    log(RayonEvent::Child(sequential_task_id));
    // end current task
    log(RayonEvent::TaskEnd(precise_time_ns()));
    // execute full sequential task
    log(RayonEvent::TaskStart(sequential_task_id, precise_time_ns()));
    log(RayonEvent::Tag(work_type, work_amount));
    let r = op();
    log(RayonEvent::Child(continuation_task_id));
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
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = |c| {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a(c);
        log(RayonEvent::Child(id_c));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_b = next_task_id();
    let cb = |c| {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b(c);
        log(RayonEvent::Child(id_c));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    log(RayonEvent::Child(id_a));
    log(RayonEvent::Child(id_b));

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
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = || {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a();
        log(RayonEvent::Child(id_c));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    let id_b = next_task_id();
    let cb = || {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b();
        log(RayonEvent::Child(id_c));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    };

    log(RayonEvent::Child(id_a));
    log(RayonEvent::Child(id_b));
    log(RayonEvent::TaskEnd(precise_time_ns()));
    let r = rayon::join(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

// small global counter to increment file names
static INSTALL_COUNT: AtomicUsize = AtomicUsize::new(0);

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
        Comparator {
            labels: Vec::new(),
            logs: Vec::new(),
            pool: self,
            runs_number: 100,
            display_preferences: Vec::new(),
        }
    }
}
/// This struct implements a pseudo builder pattern for multi-way comparisons in a single file.
pub struct Comparator<'a> {
    labels: Vec<String>,
    logs: Vec<Vec<RunLog>>,
    pool: &'a ThreadPool,
    runs_number: usize,
    display_preferences: Vec<bool>,
}

impl<'a> Comparator<'a> {
    /// PRECONDITION: call that BEFORE attaching algorithms
    pub fn runs_number(self, runs_wanted: usize) -> Self {
        Comparator {
            labels: self.labels,
            logs: self.logs,
            pool: self.pool,
            runs_number: runs_wanted,
            display_preferences: self.display_preferences,
        }
    }

    fn record_experiments<F: FnMut() -> RunLog>(&self, run_function: F) -> Vec<RunLog> {
        let mut experiments_logs: Vec<_> =
            repeat_with(run_function).take(self.runs_number).collect();
        experiments_logs.sort_unstable_by_key(|run| run.duration);
        experiments_logs
    }
    /// A copy of the following method except that it does not generate an SVG.
    pub fn attach_algorithm_nodisplay<A, STR>(mut self, label: STR, algorithm: A) -> Self
    where
        A: Fn() + Send + Sync,
        STR: Into<String>,
    {
        let logs = self.record_experiments(|| self.pool.logging_install(&algorithm).1);
        self.logs.push(logs);
        self.labels.push(label.into());
        self.display_preferences.push(false);
        self
    }
    /// Use this method for attaching an algorithm to the comparator. The algorithm will be taken
    /// as a closure and run as is.
    pub fn attach_algorithm<A, STR>(mut self, label: STR, algorithm: A) -> Self
    where
        A: Fn() + Send + Sync,
        STR: Into<String>,
    {
        let logs = self.record_experiments(|| self.pool.logging_install(&algorithm).1);
        self.logs.push(logs);
        self.labels.push(label.into());
        self.display_preferences.push(true);
        self
    }

    /// This will not create an SVG for the algorithm in the comparator.

    pub fn attach_algorithm_nodisplay_with_setup<A, I, S, T, STR>(
        mut self,
        label: STR,
        mut setup_function: S,
        algorithm: A,
    ) -> Self
    where
        S: FnMut() -> I,
        I: Send,
        A: Fn(I) -> T + Send + Sync,
        T: Send + Sync,
        STR: Into<String>,
    {
        let logs = self.record_experiments(|| {
            let input = setup_function();
            self.pool.logging_install(|| algorithm(input)).1
        });
        self.logs.push(logs);
        self.labels.push(label.into());
        self.display_preferences.push(false);
        self
    }

    /// This method lets you attach an algorithm with a setup function that will be run each time
    /// the algorithm is run. The output of the setup function will be given to the algorithm as
    /// the input.
    pub fn attach_algorithm_with_setup<A, I, S, T, STR>(
        mut self,
        label: STR,
        mut setup_function: S,
        algorithm: A,
    ) -> Self
    where
        S: FnMut() -> I,
        I: Send,
        A: Fn(I) -> T + Send + Sync,
        T: Send + Sync,
        STR: Into<String>,
    {
        let logs = self.record_experiments(|| {
            let input = setup_function();
            self.pool.logging_install(|| algorithm(input)).1
        });
        self.logs.push(logs);
        self.labels.push(label.into());
        self.display_preferences.push(true);
        self
    }

    /// This method should be called in the end to write the logs to a desired html file.
    pub fn generate_logs<P: AsRef<Path>>(self, filename: P) -> Result<(), Error> {
        let mut html_file = File::create(filename)?;

        write!(html_file, "<!DOCTYPE html>")?;
        write!(html_file, "<html><body><center>")?;
        let (last_label, first_labels) = self.labels.split_last().expect("not enough experiments");
        write!(
            html_file,
            "<H1> Comparing {} and {}</H1>",
            first_labels.join(", "),
            last_label
        )?;

        write!(
            html_file,
            "<H2>Distribution of execution times over {} runs ",
            self.runs_number
        )?;
        for (label, color) in self.labels.iter().zip(HISTOGRAM_COLORS.iter().cycle()) {
            write!(html_file, "{} is {}, ", color, label)?;
        }
        write!(html_file, "</H2>")?;
        histogram(&mut html_file, &self.logs, 30)?;
        let number_of_threads = self.logs[0][0].threads_number as f64;
        let statistics =
            Stats::get_statistics(&self.logs, number_of_threads, self.runs_number as f64);
        write!(html_file, "<H2> The Mean statistics are</H2>")?;
        write!(
            html_file,
            "<table><tr><th>algorithm</th><th>net time</th><th>sequential times</th><th>idle time</th></tr>",
        )?;
        for (name, total_time, sequential_times, idle_time) in izip!(
            self.labels.iter(),
            statistics.total_times(),
            statistics.sequential_times(),
            statistics.idle_times()
        ) {
            write!(
                html_file,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                name,
                total_time,
                compute_sequential_times_string(&sequential_times),
                idle_time
            )?;
        }
        write!(html_file, "</table>",)?;
        write!(html_file, "<H2> The Median statistics are</H2>")?;
        write!(
            html_file,
            "<table><tr><th>algorithm</th><th>net time</th><th>sequential times</th><th>idle time</th></tr>",
        )?;
        for (name, total_time, sequential_times, idle_time) in izip!(
            self.labels.iter(),
            statistics.total_times_median(),
            statistics.sequential_times_median(),
            statistics.idle_times_median()
        ) {
            write!(
                html_file,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                name,
                total_time,
                compute_sequential_times_string(&sequential_times),
                idle_time
            )?;
        }
        write!(html_file, "</table>",)?;
        write!(html_file, "<H2>Comparing median runs</H2>")?;
        let median_index = (self.runs_number) / 2;
        let speeds = compute_speeds(
            self.logs
                .iter()
                .flat_map(|row| &row[median_index].tasks_logs),
        );
        for (pos, log) in self.logs.iter().enumerate() {
            if self.display_preferences[pos] {
                let scene = visualisation(&log[median_index], Some(&speeds));
                fill_svg_file(&scene, &mut html_file)?;
                writeln!(html_file, "<p>")?;
            } else {
                continue;
            }
        }
        self.logs
            .iter()
            .zip(self.labels.iter())
            .for_each(|(algorithm, name)| {
                write!(
                    html_file,
                    "The average speeds for median run of {} are {}<br>",
                    name,
                    compute_avg_speeds(&algorithm[median_index].tasks_logs, 0, &speeds)
                )
                .expect("avg speeds failed");
            });
        write!(html_file, "<H2>Comparing best runs</H2>")?;
        let speeds = compute_speeds(self.logs.iter().flat_map(|row| &row[0].tasks_logs));
        for log in &self.logs {
            let scene = visualisation(&log[0], Some(&speeds));
            fill_svg_file(&scene, &mut html_file)?;
            writeln!(html_file, "<p>")?;
        }
        self.logs
            .iter()
            .zip(self.labels.iter())
            .for_each(|(algorithm, name)| {
                write!(
                    html_file,
                    "The average speeds for best run of {} are {}<br>",
                    name,
                    compute_avg_speeds(&algorithm[0].tasks_logs, 0, &speeds)
                )
                .expect("avg speeds failed");
            });
        write!(html_file, "</body></html>")?;
        Ok(())
    }
}

fn compute_sequential_times_string(times: &HashMap<usize, f64>) -> String {
    let mut keys: Vec<usize> = times.keys().cloned().collect();
    keys.sort_unstable();
    keys.iter()
        .map(|key| format!("{}:{}", key, times[key]))
        .join(", ")
}
