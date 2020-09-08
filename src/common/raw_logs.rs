//! Access to all logs from all threads.
use super::raw_events::{RawEvent, SubGraphId};
/// Raw unprocessed logs. Very fast to record but require some postprocessing to be displayed.
pub(crate) struct RawLogs {
    pub(crate) thread_events: Vec<Vec<RawEvent<SubGraphId>>>,
    pub(crate) labels: Vec<String>,
}
