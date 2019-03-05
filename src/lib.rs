//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
//! It also doubles down as a debugging tool.
#![type_length_limit = "2097152"] // it seems we have types with long names
#![deny(missing_docs)]
#![warn(clippy::all)]

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

mod fork_join_graph;
pub use crate::fork_join_graph::visualisation;
pub(crate) mod svg;
pub use crate::{svg::fill_svg_file, svg::write_svg_file, svg::Rectangle};
mod log;
pub use crate::log::{RunLog, TaskLog};
mod rayon_algorithms;

pub(crate) mod raw_events;
