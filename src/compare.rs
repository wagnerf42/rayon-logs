//! `Comparator` Structure for easy comparisons of different algorithms.
use crate::fork_join_graph::compute_speeds;
use crate::stats::Stats;
use crate::ThreadPool;
use crate::{
    log::RunLog,
    svg::{histogram, HISTOGRAM_COLORS},
};
use crate::{svg::fill_svg_file, visualisation};
use itertools::{izip, Itertools};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::iter::repeat_with;
use std::path::Path;

/// The comparator structure enables you to easily compare performances of different algorithms.
///
/// It runs each algorithm several times before displaying some simple statistics and for each
/// algorithm the median and best execution trace.
/// See for example the `filter_collect` example.
pub struct Comparator<'a> {
    labels: Vec<String>,
    logs: Vec<Vec<RunLog>>,
    pool: &'a ThreadPool,
    runs_number: usize,
    display_preferences: Vec<bool>,
}

impl<'a> Comparator<'a> {
    pub(crate) fn new(pool: &'a ThreadPool) -> Self {
        Comparator {
            labels: Vec::new(),
            logs: Vec::new(),
            pool,
            runs_number: 100,
            display_preferences: Vec::new(),
        }
    }
    /// Sets the number of runs for each algorithm.
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

    /// Log an algorithm's performances but do not generate svg traces.
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
    /// Log an algorithm's performances and generate svg traces.
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

    /// Log an algorithm but prepare an input (un-timed) for each execution.
    /// No svg traces.
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

    /// Log an algorithm but prepare an input (un-timed) for each execution.
    /// With svg traces.
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
        if first_labels.len() > 0 {
            // If there are more than 1 algo to compare
            write!(
                html_file,
                "<H1> Comparing {} and {}</H1>",
                first_labels.join(", "),
                last_label
            )?;
        } else {
            // If there is a single algo
            write!(html_file, "<H1> Comparing {}</H1>", last_label)?;
        }

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
        let number_of_threads = self.logs[0][0].threads_number;
        let statistics = Stats::get_statistics(&self.logs, number_of_threads, self.runs_number);
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
        if self.display_preferences.iter().any(|b| *b) {
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
                }
            }

            write!(html_file, "<H2>Comparing best runs</H2>")?;
            let speeds = compute_speeds(self.logs.iter().flat_map(|row| &row[0].tasks_logs));
            for (pos, log) in self.logs.iter().enumerate() {
                if self.display_preferences[pos] {
                    let scene = visualisation(&log[0], Some(&speeds));
                    fill_svg_file(&scene, &mut html_file)?;
                    writeln!(html_file, "<p>")?;
                }
            }
            write!(html_file, "</body></html>")?;
        }
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