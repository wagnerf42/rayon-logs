//! Provide structures holding all logged information for all tasks.
use fork_join_graph::visualisation;
use itertools::Itertools;
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::iter::once;
use std::iter::repeat;
use std::path::Path;
use svg::write_svg_file;
use {LoggedPool, RayonEvent, TaskId, TimeStamp};

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
    /// a usize tag identifying what kind of things we actually do (if known)
    /// and a u64 identifying how much work we do.
    pub work: Option<(usize, usize)>,
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
}

impl RunLog {
    /// Create a real log from logged events and reset the pool.
    pub(crate) fn new(pool: &LoggedPool) -> Self {
        let tasks_number = pool.next_task_id();
        let mut tasks_info: Vec<_> = (0..tasks_number)
            .map(|_| TaskLog {
                start_time: 0, // will be filled later
                end_time: 0,
                thread_id: 0,
                children: Vec::new(),
                work: None,
            })
            .collect();

        let iterators_number = pool.next_iterator_id();
        let mut iterators_info: Vec<_> = (0..iterators_number).map(|_| Vec::new()).collect();
        let mut iterators_fathers = Vec::new();
        // links to tasks created by join are tricky to recompute
        // when a join happens we immediately stop currently running task
        // execute the join and then create a new task "resuming" the stopped one
        // this new task has two ancestors in the dag.
        // these ancestors might be the two tasks from the join OR some tasks created down the road
        // by further joins.
        //                    0
        //              1            2
        //          4     5        7   8
        //             6             9
        //                    3
        //  at first when 0 joins 1 and 2 the resume task (3) is set to be the child of 1 and 2
        //  but when later on 1 and 2 are re-decomposed by a join we need to update this
        //  information.
        //  this dynamic information is stored in the following hashmap:
        let mut dag_children: HashMap<TaskId, TaskId> = HashMap::new();

        let threads_number = pool.tasks_logs.len();
        // remember the active task on each thread
        let mut all_active_tasks: Vec<Option<TaskId>> = repeat(None).take(threads_number).collect();

        for (thread_id, event) in pool.tasks_logs
            .iter()
            .enumerate()
            .map(|(thread_id, thread_log)| thread_log.logs().map(move |log| (thread_id, log)))
            .kmerge_by(|a, b| a.1.time() < b.1.time())
        {
            let active_tasks = &mut all_active_tasks[thread_id];
            match *event {
                RayonEvent::Join(a, b, c) => {
                    if let Some(active_task) = active_tasks {
                        tasks_info[*active_task].children.push(a); //create direct links with children
                        tasks_info[*active_task].children.push(b);

                        let possible_child = dag_children.remove(active_task); // we were set as father of someone
                        if let Some(child) = possible_child {
                            // it is not the case anymore since we are interrupted
                            dag_children.insert(c, child);
                        }
                    }
                    dag_children.insert(a, c); // a and b might be fathers of c (they are for now)
                    dag_children.insert(b, c);
                }
                RayonEvent::TaskEnd(time) => {
                    let task = active_tasks.take().unwrap();
                    tasks_info[task].end_time = time - pool.start;
                    let possible_child = dag_children.remove(&task);
                    if let Some(child) = possible_child {
                        tasks_info[task].children.push(child);
                    }
                }
                RayonEvent::TaskStart(task, time) => {
                    tasks_info[task].thread_id = thread_id;
                    tasks_info[task].start_time = time - pool.start;
                    *active_tasks = Some(task);
                }
                RayonEvent::IteratorTask(task, iterator, part, continuing_task) => {
                    let start = if let Some((start, _)) = part {
                        start
                    } else {
                        0
                    };
                    tasks_info[task].children.push(continuing_task);
                    tasks_info[task].work = part.map(|(s, e)| (iterator, e - s));
                    iterators_info[iterator].push((task, start));
                }
                RayonEvent::IteratorStart(iterator) => {
                    if let Some(active_task) = active_tasks {
                        iterators_fathers.push((iterator, *active_task));
                    }
                }
                RayonEvent::Work(work_type, work_amount) => {
                    if let Some(active_task) = active_tasks {
                        assert!(tasks_info[*active_task].work.is_none());
                        tasks_info[*active_task].work = Some((work_type, work_amount));
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

        RunLog {
            threads_number,
            tasks_logs: tasks_info,
        }
    }

    /// Load a rayon_logs log file and deserializes it into a `RunLog`.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<RunLog, io::Error> {
        let file = File::open(path).unwrap();
        serde_json::from_reader(file).map_err(|_| ErrorKind::InvalidData.into())
    }

    /// Save an svg file of all logged information.
    pub fn save_svg<P: AsRef<Path>>(
        &self,
        width: u32,
        height: u32,
        duration: u32,
        path: P,
    ) -> Result<(), io::Error> {
        let (rectangles, edges) = visualisation(once(self));
        write_svg_file(&rectangles, &edges, width, height, duration, path)
    }

    /// Save log file of currently recorded tasks logs.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let file = File::create(path)?;
        serde_json::to_writer(file, &self).expect("failed serializing");
        Ok(())
    }
}
