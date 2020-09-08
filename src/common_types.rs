//! Types which are common between rayon and rayon-logs.

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

/// Raw unprocessed logs. Very fast to record but require some postprocessing to be displayed.
pub(crate) struct RawLogs {
    pub(crate) thread_events: Vec<Vec<RawEvent<SubGraphId>>>,
    pub(crate) labels: Vec<String>,
}
