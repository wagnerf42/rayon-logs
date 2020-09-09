//! Example for recursive max with hardware events logging.
#[cfg(feature = "perf")]
fn main() {
    use rayon::join;
    use rayon_core::Logger;
    use rayon_logs::subgraph_hardware_event;
    use rayon_logs::HardwareEventType;

    fn manual_max(slice: &[u32]) -> u32 {
        if slice.len() < 200_000 {
            subgraph_hardware_event("cache_misses", HardwareEventType::CacheMisses, || {
                subgraph_hardware_event("cpu_cycles", HardwareEventType::CPUCycles, || {
                    slice.iter().max().cloned().unwrap()
                })
            })
        } else {
            let middle = slice.len() / 2;
            let (left, right) = slice.split_at(middle);
            let (mleft, mright) = join(|| manual_max(left), || manual_max(right));
            std::cmp::max(mleft, mright)
        }
    }
    let v: Vec<u32> = (0..2_000_000).collect();

    let mut logger = Logger::new();
    logger.pool_builder().build_global().unwrap();
    let max = manual_max(&v);
    assert_eq!(max, v.last().cloned().unwrap());

    logger
        .save_raw_logs("hardware_max.rlog")
        .expect("saving log file failed");
}

#[cfg(not(feature = "perf"))]
fn main() {
    eprintln!("please compile me with 'perf' feature enabled!")
}
