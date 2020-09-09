//! `LoggedPool` structure for logging raw tasks events.
use super::time_string;
use std::{collections::HashMap, iter::repeat};

// use crate::fork_join_graph::{create_graph, Block};
use crate::{log::RunLog, TimeStamp};

/// This struct mainly supplies the methods that can be used to get various statistics.
pub struct Stats<'a> {
    /// This is a slice of algorithms, for each algorithm, there is a vector of RunLogs.
    /// The vector contains one RunLog for each run of the algorithm, as per runs_number in the
    /// pool.
    logs: &'a [Vec<RunLog>],
    threads_number: usize,
    runs_number: usize,
    /// for each algorithm associate to each tag a vec of stats per run.
    /// This is an n-tuple (count, duration, normalised_speed)
    tagged_stats: Vec<HashMap<String, Vec<(usize, u64, f64)>>>,
}

impl<'l> Stats<'l> {
    /// This method returns a statistics object.
    // logs given to this function are already sorted as per wall-time.
    pub(crate) fn get_statistics(
        logs: &'l Vec<Vec<RunLog>>,
        threads_number: usize,
        runs_number: usize,
    ) -> Self {
        let tagged_stats = logs
            .iter()
            .map(|algorithm| {
                let mut tag_stats: HashMap<String, Vec<(usize, u64, f64)>> = HashMap::new();
                for run in algorithm {
                    let stats = run.stats();
                    for (key, value) in stats {
                        tag_stats.entry(key).or_default().push(value)
                    }
                }
                // we pre-sort for median
                // [BUGFIX]: This sorting is wrong, we already get the logs sorted in the order of
                // walltime. This ordering is not disturbed until now in this function, and should
                // stay that way.
                //
                //tag_stats
                //    .values_mut()
                //    .for_each(|v| v.sort_by_key(|nple| nple.1));
                tag_stats
            })
            .collect();
        Stats {
            logs,
            threads_number,
            runs_number,
            tagged_stats,
        }
    }

    /// This returns the total time summed across all runs for all experiments.
    pub fn total_times<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        self.logs
            .iter()
            .map(|algorithm| algorithm.iter().map(|run| run.duration).sum())
            .map(move |total_runs_duration: u64| total_runs_duration / self.runs_number as u64)
    }

    /// This iterates on strings for html table in compare.
    pub fn average_tagged_times<'a>(
        &'a self,
        tags: &'a [String],
    ) -> impl Iterator<Item = String> + 'a {
        self.tagged_stats.iter().map(move |algorithm| {
            tags.iter()
                .map(|t| {
                    algorithm
                        .get(t)
                        .map(|times| {
                            times.iter().map(|nple| nple.1).sum::<u64>() / self.runs_number as u64
                        })
                        .unwrap_or(0)
                })
                .map(|t| format!("<td>{}</td>", time_string(t)))
                .collect::<String>()
        })
    }

    /// Splits a table cell into three, to print all stats
    pub fn median_tagged_allstats<'a>(
        &'a self,
        tags: &'a [String],
    ) -> impl Iterator<Item = String> + 'a {
        self.tagged_stats.iter().map(move |algorithm| {
            tags.iter()
                .map(|t| {
                    algorithm
                        .get(t)
                        .map(|times| times[self.runs_number / 2])
                        .unwrap_or((0, 0, 0.0))
                })
                .map(|t| {
                    format!(
                        "<td><table><tr><td>{}</td><td>{}</td><td>{}</td></tr></table></td>",
                        t.0,
                        time_string(t.1),
                        t.2
                    )
                })
                .collect::<String>()
        })
    }
    //    /// Return the number of succesfull steals (tasks which moved between threads).
    //    pub fn succesfull_average_steals<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = usize> + 'a {
    //        self.logs.iter().map(move |algorithm| {
    //            algorithm
    //                .iter()
    //                .map(|run| {
    //                    run.tasks_logs
    //                        .iter()
    //                        .filter(|&t| t.children.len() == 2)
    //                        .map(|t| {
    //                            t.children
    //                                .iter()
    //                                .filter(|&c| run.tasks_logs[*c].thread_id != t.thread_id)
    //                                .count()
    //                        })
    //                        .sum::<usize>()
    //                })
    //                .sum::<usize>()
    //                / self.runs_number
    //        })
    //    }
    //
    pub fn tasks_split_median<'a, 'b: 'a>(
        &'b self,
        tags: &'a [String],
    ) -> impl Iterator<Item = String> + 'a {
        self.logs.iter().map(move |algorithm| {
            let count = algorithm[self.runs_number / 2].count_tasks();
            tags.iter()
                .map(|tag| count.get(tag.as_str()).copied().unwrap_or(0))
                .map(|v| format!("<td>{}</td>", v))
                .collect::<String>()
        })
    }

    pub fn get_median_task_counts<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = usize> + 'a {
        self.logs
            .iter()
            .map(move |alg| alg[self.runs_number / 2].tasks_logs.iter().count())
    }

    /// This returns the idle time summed across all runs for all experiments.
    pub fn idle_times<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        let tasks_times = self
            .logs
            .iter()
            .map(|algorithm| {
                algorithm
                    .iter()
                    .map(move |run| run.tasks_logs.iter().map(|log| log.duration()).sum::<u64>())
                    .sum()
            })
            .map(move |total_tasks_times: u64| total_tasks_times / self.runs_number as u64);
        self.total_times()
            .zip(tasks_times)
            .map(move |(duration, activity)| duration * self.threads_number as u64 - activity)
    }

    /// This returns the total time for the median runs for all experiments.
    pub fn total_times_median<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        self.logs
            .iter()
            .map(move |algorithm| algorithm[self.runs_number / 2].duration as u64)
    }

    /// This is the area of the Gantt chart of the median run of each algorithm.
    /// Ideally this should be exactly equal to sum of all task log durations over all threads, and
    /// the idle times. However, there may be some logging overheads in task log creation and the
    /// thread local variable update, which will cause a difference. Should be an interesting
    /// measure.
    pub fn unrolled_times_median<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        self.total_times_median()
            .map(move |algorithm_time| algorithm_time * self.threads_number as u64)
    }

    /// This returns the idle time for the median run for all experiments.
    /// Goes deep inside the execution trace and computes the regions of inactivity for each
    /// thread, then sums it up
    pub fn idle_times_median<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        self.logs.iter().map(move |algorithm| {
            let tasks = algorithm[self.runs_number / 2].tasks_logs.clone();
            // do one pass to figure out the last recorded time.
            // we need it to figure out who is idle at the end.
            let last_time = tasks.iter().map(|t| t.end_time).max().unwrap();
            let first_time = tasks.iter().map(|t| t.start_time).min().unwrap();

            // sort everyone by time (yes i know, again).
            // we add fake tasks at the end for last idle periods.
            let mut sorted_tasks: Vec<(usize, TimeStamp, TimeStamp)> = tasks
                .iter()
                .map(|t| (t.thread_id, t.start_time, t.end_time))
                .chain((0..self.threads_number).map(|i| (i, last_time, last_time + 1)))
                .collect();

            sorted_tasks.sort_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap());

            let mut previous_activities: Vec<TimeStamp> =
                repeat(first_time).take(self.threads_number).collect();
            let mut inactivities = 0;

            // replay execution, figuring out idle times
            for (thread_id, start, end) in sorted_tasks {
                let previous_end = previous_activities[thread_id];
                if start > previous_end {
                    inactivities += start - previous_end;
                }
                previous_activities[thread_id] = end;
            }
            inactivities
        })
    }
}
