//! `Comparator` Structure for easy comparisons of different algorithms.
use crate::fork_join_graph::compute_speeds;
use crate::stats::Stats;
use crate::ThreadPool;
use crate::{svg::fill_svg_file, visualisation};
use crate::{
    svg::{histogram, HISTOGRAM_COLORS},
    RunLog,
};
use itertools::{izip, Itertools};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::iter::repeat_with;
use std::path::Path;

/// This struct implements a pseudo builder pattern for multi-way comparisons in a single file.
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
