//! Example for recursive max with hardware events logging.
#[cfg(feature = "perf")]
fn main() {
    use rayon_logs::HardwareEventType;
    use rayon_logs::{join, subgraph_hardware_event, ThreadPoolBuilder};

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

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (max, log) = pool.logging_install(|| manual_max(&v));
    assert_eq!(max, v.last().cloned().unwrap());

    log.save_svg("hardware_max.svg")
        .expect("saving svg file failed");
    println!("saved \"hardware_max.svg\"");
    println!("hover mouse over tasks to get logged information !");
}

#[cfg(not(feature = "perf"))]
fn main() {
    eprintln!("please compile me with 'perf' feature enabled!")
}
