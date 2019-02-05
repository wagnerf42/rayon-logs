//! Provide structures holding all logged information for all tasks.
use fork_join_graph::visualisation;
use itertools::Itertools;
use serde_json;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::iter::repeat;
use std::path::Path;
use std::sync::Arc;
use svg::write_svg_file;
use {storage::Storage, RayonEvent, TaskId, TimeStamp};

/// The final information produced for log viewers.
/// A 'task' here is not a rayon task but a subpart of one.
/// a simple example with just one call to join will create
/// 4 tasks:
/// - the sequential code executed before the join
/// - the 2 join tasks
/// - the sequential code executed after the join.
/// The set of all tasks form a fork join graph.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskLog {
    /// starting time (in ns after pool creation)
    pub start_time: TimeStamp,
    /// ending time (in ns after pool creation)
    pub end_time: TimeStamp,
    /// id of thread who ran us
    pub thread_id: usize,
    /// indices of children tasks (either when forking or joining).
    pub children: Vec<TaskId>,
    /// work field may identify the task to be of iterator type, sequential type or simply untagged
    /// task. In case it is tagged to be either of the above, it will contain an ordered pair that
    /// denotes the (id, amount of work done).
    pub work: WorkInformation,
}

impl TaskLog {
    /// Return how much time it took to run this task.
    pub fn duration(&self) -> u64 {
        self.end_time - self.start_time
    }
}

/// For some tasks we know a work amount. We might know it from an iterator or from a user tag
/// using `sequential_task`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkInformation {
    IteratorWork((usize, usize)),
    SequentialWork((usize, usize)),
    NoInformation,
}

/// Store information on all tasks and threads number.
/// We also store threads number because sometimes all threads
/// are not used.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunLog {
    /// total number of threads (some might be unused).
    pub threads_number: usize,
    /// fork-join tasks.
    pub tasks_logs: Vec<TaskLog>,
    /// total run time in nanoseconds.
    pub duration: u64,
    /// all strings used for tagging tasks.
    pub tags: Vec<String>,
}

impl RunLog {
    /// Create a real log from logged events and reset the pool.
    pub(crate) fn new(
        tasks_number: usize,
        iterators_number: usize,
        tasks_logs: &[Arc<Storage>],
        start: TimeStamp,
    ) -> Self {
        let mut seen_tags = HashMap::new(); // associate each take to a usize index
        let mut tags = Vec::new(); // vector containing all tags strings
        let mut tasks_info: Vec<_> = (0..tasks_number)
            .map(|_| TaskLog {
                start_time: 0, // will be filled later
                end_time: 0,
                thread_id: 0,
                children: Vec::new(),
                work: WorkInformation::NoInformation,
            })
            .collect();

        let mut iterators_info: Vec<_> = (0..iterators_number).map(|_| Vec::new()).collect();
        let mut iterators_fathers = Vec::new();

        let threads_number = tasks_logs.len();
        // remember the active task on each thread
        let mut all_active_tasks: Vec<Option<TaskId>> = repeat(None).take(threads_number).collect();

        for (thread_id, event) in tasks_logs
            .iter()
            .enumerate()
            .map(|(thread_id, thread_log)| thread_log.logs().map(move |log| (thread_id, log)))
            .kmerge_by(|a, b| a.1.time() < b.1.time())
        {
            let active_tasks = &mut all_active_tasks[thread_id];
            match *event {
                RayonEvent::Child(c) => {
                    let father = active_tasks.expect("child with no active task as father");
                    tasks_info[father].children.push(c);
                }
                RayonEvent::TaskEnd(time) => {
                    let task = active_tasks.take().unwrap();
                    tasks_info[task].end_time = time - start;
                }
                RayonEvent::TaskStart(task, time) => {
                    tasks_info[task].thread_id = thread_id;
                    tasks_info[task].start_time = time - start;
                    *active_tasks = Some(task);
                }
                RayonEvent::IteratorTask(task, iterator, part, continuing_task) => {
                    let start = if let Some((start, _)) = part {
                        start
                    } else {
                        0
                    };
                    tasks_info[task].children.push(continuing_task);
                    tasks_info[task].work = part
                        .map(|(s, e)| WorkInformation::IteratorWork((iterator, e - s)))
                        .unwrap_or(WorkInformation::NoInformation);
                    iterators_info[iterator].push((task, start));
                }
                RayonEvent::IteratorStart(iterator) => {
                    if let Some(active_task) = active_tasks {
                        iterators_fathers.push((iterator, *active_task));
                    }
                }
                RayonEvent::Tag(work_type, work_amount) => {
                    if let Some(active_task) = active_tasks {
                        let existing_tag = seen_tags.entry(work_type);
                        let tag_index = match existing_tag {
                            Entry::Occupied(o) => *o.get(),
                            Entry::Vacant(v) => {
                                let index = tags.len();
                                v.insert(index);
                                tags.push(work_type.to_string());
                                index
                            }
                        };
                        tasks_info[*active_task].work =
                            WorkInformation::SequentialWork((tag_index, work_amount));
                    } else {
                        panic!("tagging a non existing task");
                    }
                }
            }
        }

        // now parse iterator info to link iterator tasks to graph
        for (iterator, father) in &iterators_fathers {
            let mut children = &mut iterators_info[*iterator];
            children.sort_unstable_by_key(|(_, start)| *start);
            tasks_info[*father]
                .children
                .extend(children.iter().map(|(task, _)| task));
        }

        let duration = tasks_info.iter().map(|t| t.end_time).max().unwrap()
            - tasks_info.iter().map(|t| t.start_time).min().unwrap();

        RunLog {
            threads_number,
            tasks_logs: tasks_info,
            duration,
            tags,
        }
    }

    /// Load a rayon_logs log file and deserializes it into a `RunLog`.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<RunLog, io::Error> {
        let file = File::open(path).unwrap();
        serde_json::from_reader(file).map_err(|_| ErrorKind::InvalidData.into())
    }

    /// Save an svg file of all logged information.
    pub fn save_svg<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let scene = visualisation(self, None);
        write_svg_file(&scene, path)
    }

    /// Save log file of currently recorded tasks logs.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let file = File::create(path)?;
        serde_json::to_writer(file, &self).expect("failed serializing");
        Ok(())
    }
}
