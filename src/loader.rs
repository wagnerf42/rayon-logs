//! Functions for loading log files.
use crate::log::RunLog;
use crate::{RawEvent, RawLogs, SubGraphId, TaskId};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io;
use std::path::Path;

pub fn load_raw_logs<P: AsRef<Path>>(path: P) -> Result<RawLogs, io::Error> {
    let mut file = File::open(path)?;
    // read vector of strings constituting labels
    let labels = read_vec_strings_from(&mut file)?;
    // read number of threads
    let threads_number = file.read_u64::<LittleEndian>()? as usize;
    // read all events
    let thread_events = std::iter::repeat_with(|| {
        let events_number = file.read_u64::<LittleEndian>()? as usize;
        std::iter::repeat_with(|| read_event_from(&mut file))
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
/// Convert given log file to interactive svg file.
pub fn log2svg<P: AsRef<Path>, Q: AsRef<Path>>(log_path: P, svg_path: Q) -> Result<(), io::Error> {
    let raw_logs = load_raw_logs(log_path)?;
    let run_log = RunLog::new(raw_logs);
    run_log.save_svg(svg_path)?;
    Ok(())
}

pub(crate) fn read_event_from<R: std::io::Read>(
    source: &mut R,
) -> std::io::Result<RawEvent<SubGraphId>> {
    let mut byte = [0u8];
    source.read_exact(&mut byte)?;
    let event = match byte[0] {
        2 => RawEvent::TaskStart(
            source.read_u64::<LittleEndian>()? as TaskId,
            source.read_u64::<LittleEndian>()?,
        ),
        3 => RawEvent::TaskEnd(source.read_u64::<LittleEndian>()?),
        4 => RawEvent::Child(source.read_u64::<LittleEndian>()? as TaskId),
        5 => RawEvent::SubgraphStart(source.read_u64::<LittleEndian>()? as SubGraphId),
        6 => RawEvent::SubgraphEnd(
            source.read_u64::<LittleEndian>()? as SubGraphId,
            source.read_u64::<LittleEndian>()? as usize,
        ),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid event discriminant",
            ))
        }
    };
    Ok(event)
}
