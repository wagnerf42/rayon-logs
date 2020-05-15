//! `LoggedPool` structure for logging raw tasks events.
use std::collections::HashMap;

// use crate::fork_join_graph::{create_graph, Block};
use crate::log::RunLog;

/// This struct mainly supplies the methods that can be used to get various statistics.
pub struct Stats<'a> {
    logs: &'a [Vec<RunLog>],
    threads_number: usize,
    runs_number: usize,
    /// for each algorithm associate to each tag a vec of times per run
    tagged_times: Vec<HashMap<String, Vec<u64>>>,
}

impl<'l> Stats<'l> {
    /// This method returns a statistics object.
    pub fn get_statistics(
        logs: &'l Vec<Vec<RunLog>>,
        threads_number: usize,
        runs_number: usize,
    ) -> Self {
        let tagged_times = logs
            .iter()
            .map(|algorithm| {
                let mut hashmap: HashMap<String, Vec<u64>> = HashMap::new();
                for run in algorithm {
                    let stats = run.stats();
                    for (key, value) in stats {
                        hashmap.entry(key).or_default().push(value.1)
                    }
                }
                // we pre-sort for median
                hashmap.values_mut().for_each(|v| v.sort());
                hashmap
            })
            .collect();
        Stats {
            logs,
            threads_number,
            runs_number,
            tagged_times,
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
        self.tagged_times.iter().map(move |algorithm| {
            tags.iter()
                .map(|t| {
                    algorithm
                        .get(t)
                        .map(|times| times.iter().sum::<u64>() / self.runs_number as u64)
                        .unwrap_or(0)
                })
                .map(|t| format!("<td>{}</td>", crate::compare::time_string(t)))
                .collect::<String>()
        })
    }

    /// This iterates on strings for html table in compare.
    pub fn median_tagged_times<'a>(
        &'a self,
        tags: &'a [String],
    ) -> impl Iterator<Item = String> + 'a {
        self.tagged_times.iter().map(move |algorithm| {
            tags.iter()
                .map(|t| {
                    algorithm
                        .get(t)
                        .map(|times| times[self.runs_number / 2])
                        .unwrap_or(0)
                })
                .map(|t| format!("<td>{}</td>", crate::compare::time_string(t)))
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
    //    /// Returns an iterator over the average number of tasks that were created for each algorithm
    //    /// in the logs.
    //    pub fn tasks_count<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = usize> + 'a {
    //        self.logs.iter().map(move |algorithm| {
    //            algorithm
    //                .iter()
    //                .map(|run| {
    //                    create_graph(&run.tasks_logs)
    //                        .0
    //                        .iter()
    //                        .filter(|&b| {
    //                            if let Block::Sequence(_) = b {
    //                                true
    //                            } else {
    //                                false
    //                            }
    //                        })
    //                        .count()
    //                })
    //                .sum::<usize>()
    //                / self.runs_number
    //        })
    //    }

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

    /// This returns the idle time for the median run for all experiments.
    pub fn idle_times_median<'a, 'b: 'a>(&'b self) -> impl Iterator<Item = u64> + 'a {
        self.logs
            .iter()
            .map(move |algorithm| {
                algorithm[self.runs_number / 2]
                    .tasks_logs
                    .iter()
                    .map(|log| log.duration() as u64)
                    .sum::<u64>()
            })
            .zip(self.total_times_median())
            .map(move |(compute_time, total_time)| {
                (total_time * self.threads_number as u64) - compute_time
            })
    }
}
