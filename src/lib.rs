//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
#![deny(missing_docs)]
extern crate rayon;
extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate itertools;
extern crate serde_json;

mod iterator;
mod storage;
pub use iterator::Logged;
mod pool;
pub use pool::LoggedPool;
mod builder;
pub mod prelude;
pub use builder::LoggedPoolBuilder;
mod fork_join_graph;
pub use fork_join_graph::visualization_rectangles;
pub(crate) mod svg;
pub use svg::Rectangle;

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
    /// Tag current task with a type of work (usize id) and a work amount.
    Work(usize, usize),
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

/// The final information produced for log viewers.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskLog {
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
    /// a usize tag identifying what kind of things we actually do (if known)
    /// and a u64 identifying how much work we do.
    work: Option<(usize, usize)>,
}
