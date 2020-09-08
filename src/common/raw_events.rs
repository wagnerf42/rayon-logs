//! We define here all raw events.
//! Events which are very fast to log and logged on a per thread basis.
//! These events will be post-processed after execution in order to generate
//! a tasks graph.

/// unique thread identifier
pub(crate) type ThreadId = usize;
/// unique subgraph identifier
pub(crate) type SubGraphId = usize;
/// unique task identifier
pub(crate) type TaskId = usize;
/// at which time (in nanoseconds) does the event happen
pub(crate) type TimeStamp = u64;

/// All types of raw events we can log.
/// It is generic because recorded logs and reloaded logs
/// don't use the same strings for subgraphs.
#[derive(Debug, Clone)]
pub(crate) enum RawEvent<S> {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// Active task ends.
    TaskEnd(TimeStamp),
    /// Direct link in the graph between two tasks (active one and given one).
    Child(TaskId),
    /// Start a subgraph.
    SubgraphStart(S),
    /// End a subgraph and register a work amount.
    SubgraphEnd(S, usize),
}

impl<S> RawEvent<S> {
    pub(crate) fn time(&self) -> TimeStamp {
        match *self {
            RawEvent::TaskStart(_, t) => t,
            RawEvent::TaskEnd(t) => t,
            _ => 0,
        }
    }
}
