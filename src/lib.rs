//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
//! It also doubles down as a debugging tool.
//!
//! <div>
//! <img src="http://www-id.imag.fr/Laboratoire/Membres/Wagner_Frederic/images/downgraded_iter_sum.svg"/>
//! </div>
#![type_length_limit = "2097152"] // it seems we have types with long names
#![deny(missing_docs)]
#![warn(clippy::all)]

use rayon_core::{RawEvent, RawLogs, SubGraphId, TaskId, TimeStamp};
// everything below is rayon-logs only:
// logs postprocessing, graphs, svg,...
mod loader;
pub use loader::log2svg;

mod log;

mod comparator;
pub use crate::comparator::compare::Comparator;

mod visualisation;

#[cfg(feature = "perf")]
mod counters;
#[cfg(feature = "perf")]
pub use counters::{subgraph_cache_event, subgraph_hardware_event, subgraph_software_event};

/// We reexport perf-related types here.
#[cfg(feature = "perf")]
pub use perfcnt::linux::{
    CacheId, CacheOpId, CacheOpResultId, HardwareEventType, SoftwareEventType,
};
