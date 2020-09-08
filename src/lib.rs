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

pub(crate) mod common; // this comes first because it exports the logs macro

mod loader;
pub use loader::log2svg;
mod rayon;
pub use self::rayon::recorder::{reset, save_raw_logs};
pub use self::rayon::scope::{scope, scope_fifo, Scope, ScopeFifo};
pub use self::rayon::subgraphs::{custom_subgraph, subgraph};
pub use self::rayon::{join, join_context};

mod counters;
#[cfg(feature = "perf")]
pub use counters::{subgraph_cache_event, subgraph_hardware_event, subgraph_software_event};
mod log;
pub use log::save_svg;
mod comparator;
pub use crate::comparator::compare::Comparator;
mod visualisation;

/// We reexport perf-related types here.
#[cfg(feature = "perf")]
pub use perfcnt::linux::{
    CacheId, CacheOpId, CacheOpResultId, HardwareEventType, SoftwareEventType,
};
