//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
#![type_length_limit = "2097152"]
#![deny(missing_docs)]
#![warn(clippy::all)]

use serde_derive::{Deserialize, Serialize};
mod iterator;
mod storage;
pub use crate::iterator::Logged;
pub use crate::storage::Storage;
mod pool;
pub use crate::pool::Comparator;
pub use crate::pool::{join, join_context, make_subgraph, sequential_task, ThreadPool};
mod builder;
pub mod prelude;
pub use crate::builder::ThreadPoolBuilder;
pub use rayon::current_num_threads;
mod scope;
pub use crate::scope::{scope, Scope};

mod stats;
pub use crate::stats::Stats;

mod fork_join_graph;
pub use crate::fork_join_graph::visualisation;
pub(crate) mod svg;
pub use crate::{svg::fill_svg_file, svg::write_svg_file, svg::Rectangle};
mod log;
pub use crate::log::{RunLog, TaskLog};
mod rayon_algorithms;

type TaskId = usize;
type IteratorId = usize;
type TimeStamp = u64;

/// All types of events we can log.
#[derive(Debug, Serialize, Deserialize)]
pub enum RayonEvent {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// Active task ends.
    TaskEnd(TimeStamp),
    /// Direct link in the graph between two tasks (active one and given one).
    Child(TaskId),
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
