//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
#![type_length_limit = "2097152"]
#![deny(missing_docs)]
#![warn(clippy::all)]
extern crate rayon;
extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate itertools;
extern crate serde_json;

mod iterator;
mod storage;
pub use iterator::Logged;
mod pool;
pub use pool::Comparator;
pub use pool::{join, join_context, sequential_task, ThreadPool};
mod builder;
pub mod prelude;
pub use builder::ThreadPoolBuilder;
pub use rayon::current_num_threads;

mod stats;
pub use stats::Stats;

mod fork_join_graph;
pub use fork_join_graph::visualisation;
pub(crate) mod svg;
pub use {svg::fill_svg_file, svg::write_svg_file, svg::Rectangle};
mod log;
pub use log::{RunLog, TaskLog};
mod rayon_algorithms;

type TaskId = usize;
type IteratorId = usize;
type TimeStamp = u64;

/// All types of events we can log.
#[derive(Debug, Serialize, Deserialize)]
enum RayonEvent {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// Active task ends.
    TaskEnd(TimeStamp),
    /// We create two tasks with join (contains dependencies information) at their end we continue
    /// with another task (third id).
    Join(TaskId, TaskId, TaskId),
    /// Log additional informations for iterators tasks.
    IteratorTask(TaskId, IteratorId, Option<(usize, usize)>, TaskId),
    /// Who starts a new iterator.
    IteratorStart(IteratorId),
    /// We have a sequential task (child).
    /// id is of child and grand child.
    SequentialTask(TaskId, TaskId),
    /// Tag a task with work and type.
    Tag(usize, usize),
}

impl RayonEvent {
    /// return event time or 0 if none
    fn time(&self) -> u64 {
        match *self {
            RayonEvent::TaskStart(_, t) => t,
            RayonEvent::TaskEnd(t) => t,
            _ => 0,
        }
    }
}
