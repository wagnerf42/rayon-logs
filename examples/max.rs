extern crate rayon_logs;
use rayon_logs::{prelude::*, ThreadPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("building pool failed");

    let (max, log) = pool.install(|| v.par_iter().max().cloned().unwrap());
    assert_eq!(max, v.last().cloned().unwrap());
    log.save_svg("max.svg").expect("saving svg failed");
}
