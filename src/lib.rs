//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
#![deny(missing_docs)]
extern crate rayon;
extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod iterator;
pub use iterator::Logged;
mod pool;
pub use pool::LoggedPool;
mod builder;
pub mod prelude;
pub use builder::LoggedPoolBuilder;

type TaskId = usize;
type IteratorId = usize;
type TimeStamp = u64;

/// All types of events we can log.
#[derive(Debug, Serialize, Deserialize)]
enum RayonEvent {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// A task ends.
    TaskEnd(TaskId, TimeStamp),
    /// We create two tasks with join (contains dependencies information).
    Join(TaskId, TaskId),
    /// Log additional informations for iterators tasks.
    IteratorTask(TaskId, IteratorId, Option<(usize, usize)>),
}

/// The final information produced for log viewers.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskLog {
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
}
