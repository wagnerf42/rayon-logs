//! Structs, functions and global variables for recording logs.
use super::list::AtomicLinkedList;
use super::now;
use super::storage::Storage;
use crate::common_types::{RawEvent, RawLogs, SubGraphId, TaskId, ThreadId};
use byteorder::{LittleEndian, WriteBytesExt};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// each thread will get a unique id and increment this counter
static THREADS_COUNT: AtomicUsize = AtomicUsize::new(0);
// we need to serialize insertions in the list of storages.
static REGISTERED_THREADS_COUNT: AtomicUsize = AtomicUsize::new(0);

// we store a list of all logs together with their threads ids
static LOGS: AtomicLinkedList<(usize, Arc<Storage<RawEvent<&'static str>>>)> =
    AtomicLinkedList::new();

thread_local! {
    /// each thread has a unique id
    pub(crate) static ID: usize = THREADS_COUNT.fetch_add(1, Ordering::Relaxed);
    /// each thread has a storage space for logs
    pub(crate) static THREAD_LOGS: Arc<Storage<RawEvent<&'static str>>> =  {
        let logs = Arc::new(Storage::new());
        ID.with(|id| {
            // let's spinlock waiting for our turn
            let backoff = crossbeam::utils::Backoff::new();
            while REGISTERED_THREADS_COUNT.load(Ordering::SeqCst) != *id {
                backoff.spin()
            }
            // TODO: does main always get 0 ?
            if *id == 0 {
                logs.push(RawEvent::TaskStart(0, now()));
            }
            LOGS.push_front((*id, logs.clone()));
            REGISTERED_THREADS_COUNT.fetch_add(1, Ordering::SeqCst);
        });
        logs
    };
}

/// Erase all logs.
/// PRE-condition: call from main thread. // TODO: is this acceptable ?
pub fn reset() {
    LOGS.iter().for_each(|(_, log)| log.reset());
    crate::rayon::log(RawEvent::TaskStart(crate::rayon::next_task_id(), now()));
}

impl RawLogs {
    /// Extract recorded events and reset structs.
    /// It's better to do it when no events are being recorded.
    /// We are able to extract logs during recording but the obtained logs
    /// might be incomplete.
    /// pre-condition: call from main thread. // TODO: keep it ???
    pub(crate) fn new() -> Self {
        // stop main task
        crate::rayon::log(RawEvent::TaskEnd(now()));
        // associate a unique integer id to each label
        let mut next_label_count = 0;
        let mut seen_labels = HashMap::new();
        let mut labels = Vec::new();
        let mut events: HashMap<ThreadId, Vec<RawEvent<SubGraphId>>> = HashMap::new();
        // loop on all logged  rayon events per thread
        for &(thread_id, ref thread_logs) in LOGS.iter().sorted_by_key(|&(thread_id, _)| thread_id)
        {
            for rayon_event in thread_logs.iter() {
                // store eventual event label
                match rayon_event {
                    RawEvent::SubgraphStart(label) | RawEvent::SubgraphEnd(label, _) => {
                        seen_labels.entry(*label).or_insert_with(|| {
                            let label_count = next_label_count;
                            next_label_count += 1;
                            labels.push(label.to_string());
                            label_count
                        });
                    }
                    _ => (),
                }
                // convert to raw_event
                let raw_event = RawEvent::new(rayon_event, &seen_labels);
                events
                    .entry(thread_id)
                    .or_insert_with(Vec::new)
                    .push(raw_event);
            }
        }

        // now we just need to turn the hash table into a vector, filling the gaps
        // if some threads registered no events yet
        let threads_number = THREADS_COUNT.load(Ordering::Relaxed);
        RawLogs {
            thread_events: (0..threads_number)
                .map(|thread_id| events.remove(&thread_id).unwrap_or_else(Vec::new))
                .collect(),
            labels,
        }
    }
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let mut file = File::create(path)?;
        // we start by saving all labels
        write_vec_strings_to(&self.labels, &mut file)?;
        // write the number of threads
        file.write_u64::<LittleEndian>(self.thread_events.len() as u64)?;
        // now, all events
        for events in &self.thread_events {
            file.write_u64::<LittleEndian>(events.len() as u64)?; // how many events for this thread
            events.iter().try_for_each(|e| e.write_to(&mut file))?;
        }
        Ok(())
    }
}

fn write_vec_strings_to<W: std::io::Write>(
    vector: &Vec<String>,
    destination: &mut W,
) -> std::io::Result<()> {
    // write the length
    destination.write_u64::<LittleEndian>(vector.len() as u64)?;
    // write for each string its byte size and then all bytes
    for string in vector {
        let bytes = string.as_bytes();
        destination.write_u64::<LittleEndian>(string.len() as u64)?;
        destination.write(bytes)?;
    }
    Ok(())
}

/// Save log file of currently recorded raw logs.
/// This will reset logs.
pub fn save_raw_logs<P: AsRef<Path>>(path: P) -> Result<(), io::Error> {
    let logs = RawLogs::new();
    logs.save(path)?;
    reset();
    Ok(())
}

impl RawEvent<TaskId> {
    pub(crate) fn new(
        rayon_event: &RawEvent<&'static str>,
        strings: &HashMap<&str, usize>,
    ) -> RawEvent<TaskId> {
        match rayon_event {
            RawEvent::TaskStart(id, time) => RawEvent::TaskStart(*id, *time),
            RawEvent::TaskEnd(time) => RawEvent::TaskEnd(*time),
            RawEvent::Child(id) => RawEvent::Child(*id),
            RawEvent::SubgraphStart(label) => RawEvent::SubgraphStart(strings[label]),
            RawEvent::SubgraphEnd(label, size) => RawEvent::SubgraphEnd(strings[label], *size),
        }
    }
    pub(crate) fn write_to<W: std::io::Write>(&self, destination: &mut W) -> std::io::Result<()> {
        match self {
            RawEvent::TaskStart(id, time) => {
                destination.write(&[2u8])?;
                destination.write_u64::<LittleEndian>(*id as u64)?;
                destination.write_u64::<LittleEndian>(*time)?;
            }
            RawEvent::TaskEnd(time) => {
                destination.write(&[3u8])?;
                destination.write_u64::<LittleEndian>(*time)?;
            }
            RawEvent::Child(id) => {
                destination.write(&[4u8])?;
                destination.write_u64::<LittleEndian>(*id as u64)?;
            }
            RawEvent::SubgraphStart(label) => {
                destination.write(&[5u8])?;
                destination.write_u64::<LittleEndian>(*label as u64)?;
            }
            RawEvent::SubgraphEnd(label, size) => {
                destination.write(&[6u8])?;
                destination.write_u64::<LittleEndian>(*label as u64)?;
                destination.write_u64::<LittleEndian>(*size as u64)?;
            }
        }
        Ok(())
    }
}
