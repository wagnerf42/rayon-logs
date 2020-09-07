//! `Comparator` Structure for easy comparisons of different algorithms.
use crate::stats::Stats;
use crate::ThreadPool;
use crate::{
    log::RunLog,
    raw_logs::RawLogs,
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
    /// Renumber all tags accross all logs such that tags number match.
    /// Return vector of all tags.
    fn fuse_tags(&mut self) -> Vec<String> {
        let mut global_tags = HashMap::new();
        for experiment in &self.logs {
            for log in experiment {
                log.scan_tags(&mut global_tags);
            }
        }
        for experiment in &mut self.logs {
            for log in experiment {
                log.update_tags(&global_tags);
            }
        }
        global_tags
            .into_iter()
            .sorted_by_key(|&(_, i)| i)
            .map(|(t, _)| t)
            .collect()
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
        let logs = self.record_experiments(|| {
            self.pool.install(&algorithm);
            RunLog::new(RawLogs::new())
        });
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
        let logs = self.record_experiments(|| {
            self.pool.install(&algorithm);
            RunLog::new(RawLogs::new())
        });
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
            self.pool.install(|| algorithm(input));
            RunLog::new(RawLogs::new())
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
            self.pool.install(|| algorithm(input));
            RunLog::new(RawLogs::new())
        });
        self.logs.push(logs);
        self.labels.push(label.into());
        self.display_preferences.push(true);
        self
    }

    /// This method should be called in the end to write the logs to a desired html file.
    pub fn generate_logs<P: AsRef<Path>>(mut self, filename: P) -> Result<(), Error> {
        let tags = self.fuse_tags(); // have a consistent tags numbering accross all logs
        let mut html_file = File::create(filename)?;

        writeln!(html_file, "<!DOCTYPE html>")?;
        writeln!(
            html_file,
            r#"
<html><head><style>
table, th, td {{
  border: 1px solid black;
  border-collapse: collapse;
}}
</style>
</head>
<body><center>"#,
        )?;
        let (last_label, first_labels) = self.labels.split_last().expect("not enough experiments");
        writeln!(
            html_file,
            "<H1> Comparing {} and {}</H1>",
            first_labels.join(", "),
            last_label
        )?;

        writeln!(
            html_file,
            "<H2>Distribution of execution times over {} runs ",
            self.runs_number
        )?;
        for (label, color) in self.labels.iter().zip(HISTOGRAM_COLORS.iter().cycle()) {
            writeln!(
                html_file,
                "<text style=\"color:{0}\">{0}</text> is {1}, ",
                color, label
            )?;
        }
        writeln!(html_file, "</H2>")?;
        histogram(&mut html_file, &self.logs, 30)?;
        let number_of_threads = self.logs[0][0].threads_number;
        let statistics = Stats::get_statistics(&self.logs, number_of_threads, self.runs_number);
        writeln!(html_file, "<H2> The Mean statistics are</H2>")?;
        writeln!(
            html_file,
            "<table><tr><th></th><th>algorithm</th><th>net time</th>{}<th>idle time</th></tr>",
            tags.iter()
                .map(|t| format!("<th>{}</th>", t))
                .collect::<String>()
        )?;
        for (name, total_time, tagged_columns, idle_time, algo_color) in izip!(
            //for (name, total_time, sequential_times, idle_time, algo_color) in izip!(
            self.labels.iter(),
            statistics.total_times(),
            statistics.average_tagged_times(&tags),
            statistics.idle_times(),
            HISTOGRAM_COLORS.iter().cycle()
        ) {
            writeln!(
                html_file,
                "<tr><td>{}</td><td>{}</td><td>{}</td>{}<td>{}</td></tr>",
                format!("<span style='color:{}'>&#9632;</span>", algo_color),
                name,
                time_string(total_time),
                tagged_columns,
                time_string(idle_time)
            )?;
        }
        writeln!(html_file, "</table>",)?;
        writeln!(html_file, "<H2> The Median statistics are</H2>")?;
        writeln!(html_file, "<H4> you may see tagged statistics for your tags in the form (count, duration, speed)</H4>")?;
        writeln!(
            html_file,
            "<table><tr><th></th><th>algorithm</th><th>unrolled time</th>{}<th>idle time</th></tr>",
            tags.iter()
                .map(|t| format!("<th>{}</th>", t))
                .collect::<String>()
        )?;
        for (name, total_time, tagged_columns, idle_time, algo_color) in izip!(
            self.labels.iter(),
            statistics.unrolled_times_median(),
            statistics.median_tagged_allstats(&tags),
            statistics.idle_times_median(),
            HISTOGRAM_COLORS.iter().cycle()
        ) {
            writeln!(
                html_file,
                "<tr><td>{}</td><td>{}</td><td>{}</td>{}<td>{}</td></tr>",
                format!("<span style='color:{}'>&#9632;</span>", algo_color),
                name,
                time_string(total_time),
                tagged_columns,
                time_string(idle_time)
            )?;
        }
        writeln!(html_file, "</table>",)?;

        writeln!(html_file, "<H2> The Median task counts are</H2>")?;
        writeln!(
            html_file,
            "<table><tr><th></th><th>algorithm</th><th>total count</th>{}</tr>",
            tags.iter()
                .map(|t| format!("<th>{}</th>", t))
                .collect::<String>()
        )?;
        for (name, total_count, tagged_counts, algo_color) in izip!(
            self.labels.iter(),
            statistics.get_median_task_counts(),
            statistics.tasks_split_median(&tags),
            HISTOGRAM_COLORS.iter().cycle()
        ) {
            writeln!(
                html_file,
                "<tr><td>{}</td><td>{}</td><td>{}</td>{}</tr>",
                format!("<span style='color:{}'>&#9632;</span>", algo_color),
                name,
                total_count,
                tagged_counts,
            )?;
        }
        writeln!(html_file, "</table>",)?;
        if self.display_preferences.iter().any(|b| *b) {
            writeln!(html_file, "<H2>Comparing median runs</H2>")?;
            let median_index = (self.runs_number) / 2;
            for (pos, (log, name)) in self.logs.iter().zip(self.labels.iter()).enumerate() {
                if self.display_preferences[pos] {
                    let scene = visualisation(&log[median_index]);
                    writeln!(html_file, "<H3 align=\"left\"><u>{}</u> :</H3>", name)?;
                    fill_svg_file(&scene, &mut html_file)?;
                    writeln!(html_file, "<p>")?;
                }
            }

            writeln!(html_file, "<H2>Comparing best runs</H2>")?;
            for (pos, (log, name)) in self.logs.iter().zip(self.labels.iter()).enumerate() {
                if self.display_preferences[pos] {
                    let scene = visualisation(&log[0]);
                    writeln!(html_file, "<H3 align=\"left\"><u>{}</u> :</H3>", name)?;
                    fill_svg_file(&scene, &mut html_file)?;
                    writeln!(html_file, "<p>")?;
                }
            }
            write!(html_file, "</body></html>")?;
        }
        Ok(())
    }
}

pub(crate) fn time_string(nano: u64) -> String {
    match nano {
        n if n < 1_000 => format!("{}ns", n),
        n if n < 1_000_000 => format!("{:.2}us", (n as f64 / 1_000.0)),
        n if n < 1_000_000_000 => format!("{:.2}ms", (n as f64 / 1_000_000.0)),
        n if n < 60_000_000_000 => format!("{:.2}s", (n as f64 / 1_000_000_000.0)),
        n => format!("{}m{}s", n / 60_000_000_000, n % 60_000_000_000),
    }
}
