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
mod storage;
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
    /// Who starts a new iterator.
    IteratorStart(IteratorId),
    /// Tag current task with a type of work (usize id) and a work amount.
    Work(usize, usize),
}

/// The final information produced for log viewers.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskLog {
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
    /// a usize tag identifying what kind of things we actually do (if known)
    /// and a u64 identifying how much work we do.
    work: Option<(usize, usize)>,
}
