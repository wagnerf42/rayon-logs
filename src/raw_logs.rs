//! Access to all logs from all threads.
use crate::list::AtomicLinkedList;
use crate::raw_events::{RawEvent, RayonEvent, ThreadId};
use crate::storage::Storage;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use itertools::Itertools;
use lazy_static::lazy_static;
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
            // let's spinlock waiting for our turn
            let backoff = crossbeam::utils::Backoff::new();
            while REGISTERED_THREADS_COUNT.load(Ordering::SeqCst) != *id {
                backoff.spin()
            }
            LOGS.push_front((*id, logs.clone()));
            REGISTERED_THREADS_COUNT.fetch_add(1, Ordering::SeqCst);
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

struct RawLogs {
    thread_events: Vec<Vec<RawEvent>>,
    labels: Vec<String>,
}

impl RawLogs {
    /// Extract recorded events and reset structs.
    /// It's better to do it when no events are being recorded.
    /// We are able to extract logs during recording but the obtained logs
    /// might be incomplete.
    fn new() -> Self {
        // associate a unique integer id to each label
        let mut next_label_count = 0;
        let mut seen_labels = HashMap::new();
        let mut labels = Vec::new();
        let mut events: HashMap<ThreadId, Vec<RawEvent>> = HashMap::new();
        // loop on all logged  rayon events per thread
        for &(thread_id, ref thread_logs) in LOGS.iter().sorted_by_key(|&(thread_id, _)| thread_id)
        {
            for rayon_event in thread_logs.iter() {
                // store eventual event label
                match rayon_event {
                    RayonEvent::SubgraphStart(label) | RayonEvent::SubgraphEnd(label, _) => {
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
    fn load<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let mut file = File::open(path)?;
        // read vector of strings constituting labels
        let labels = read_vec_strings_from(&mut file)?;
        // read number of threads
        let threads_number = file.read_u64::<LittleEndian>()? as usize;
        // read all events
        let thread_events = std::iter::repeat_with(|| {
            let events_number = file.read_u64::<LittleEndian>()? as usize;
            std::iter::repeat_with(|| RawEvent::read_from(&mut file))
                .take(events_number)
                .collect::<Result<Vec<_>, _>>()
        })
        .take(threads_number)
        .collect::<Result<Vec<_>, _>>()?;
        Ok(RawLogs {
            labels,
            thread_events,
        })
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

fn read_vec_strings_from<R: std::io::Read>(source: &mut R) -> Result<Vec<String>, io::Error> {
    let size = source.read_u64::<LittleEndian>()? as usize;
    let mut strings = Vec::with_capacity(size);
    for _ in 0..size {
        let string_size = source.read_u64::<LittleEndian>()? as usize;
        let mut buffer = vec![0u8; string_size];
        source.read_exact(&mut buffer)?;
        let string =
            String::from_utf8(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        strings.push(string)
    }
    Ok(strings)
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
pub fn save_raw_logs<P: AsRef<Path>>(path: P) -> Result<(), io::Error> {
    let logs = RawLogs::new();
    reset();
    logs.save(path)?;
    Ok(())
}