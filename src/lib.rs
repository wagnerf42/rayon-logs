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
pub use pool::{join, join_context, make_subgraph, sequential_task, ThreadPool};
mod builder;
pub mod prelude;
pub use builder::ThreadPoolBuilder;
pub use rayon::current_num_threads;
mod scope;
pub use scope::{scope, Scope};

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
    /// Direct link in the graph between two tasks (active one and given one).
    Child(TaskId),
    /// Log additional informations for iterators tasks.
    IteratorTask(TaskId, IteratorId, Option<(usize, usize)>, TaskId),
    /// Who starts a new iterator.
    IteratorStart(IteratorId),
    /// Tag a subgraph with work type, work amount.
    SubgraphStart(&'static str, usize),
    /// Tag the end of a subgraph.
    SubgraphEnd(&'static str),
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
