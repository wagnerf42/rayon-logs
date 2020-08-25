//! Access to all logs from all threads.
use crate::list::AtomicLinkedList;
use crate::raw_events::{RayonEvent, ThreadId};
use crate::storage::Storage;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// each thread will get a unique id and increment this counter
static THREADS_COUNT: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    // we store a list of all logs together with their threads ids
    static ref LOGS: AtomicLinkedList<(usize, Arc<Storage<RayonEvent>>)> = AtomicLinkedList::new();
}

thread_local! {
    /// each thread has a unique id
    pub(crate) static ID: usize = THREADS_COUNT.fetch_add(1, Ordering::Relaxed);
    /// each thread has a storage space for logs
    pub(crate) static THREAD_LOGS: Arc<Storage<RayonEvent>> =  {
        let logs = Arc::new(Storage::new());
        ID.with(|id| {
            LOGS.push_front((*id, logs.clone()));
        });
        logs
    };
}

/// Loop on all recorded events from oldest to newest.
pub(crate) fn recorded_events() -> impl Iterator<Item = (ThreadId, &'static RayonEvent)> {
    LOGS.iter()
        .map(|&(thread_id, ref thread_logs)| thread_logs.iter().map(move |log| (thread_id, log)))
        .kmerge_by(|a, b| a.1.time() < b.1.time())
}

/// Erase all logs.
pub(crate) fn reset() {
    LOGS.iter().for_each(|(_, log)| log.reset())
}
