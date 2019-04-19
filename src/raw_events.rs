//! We define here all raw events.
//! Events which are very fast to log and logged on a per thread basis.
//! These events will be post-processed after execution in order to generate
//! a tasks graph.
use serde_derive::{Deserialize, Serialize};

/// unique task identifier
pub(crate) type TaskId = usize;
/// unique iterator identifier (currently unused, will come back later)
pub(crate) type IteratorId = usize;
/// at which time (in nanoseconds) does the event happen
pub(crate) type TimeStamp = u64;

/// All types of raw events we can log.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RayonEvent {
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
    /// Tag a subgraph with work type, work amount.
    SubgraphPerfStart(&'static str),
    /// Tag the end of a subgraph.
    SubgraphPerfEnd(&'static str, usize, &'static str),
}

impl RayonEvent {
    /// return event time or 0 if none
    pub(crate) fn time(&self) -> u64 {
        match *self {
            RayonEvent::TaskStart(_, t) => t,
            RayonEvent::TaskEnd(t) => t,
            _ => 0,
        }
    }
}
