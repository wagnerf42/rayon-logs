//! Basic iterator log.
extern crate rayon_logs as rayon;
use rayon::{prelude::*, ThreadPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .expect("building pool failed");
    let (max, log) = pool.logging_install(|| v.par_iter().max());
    assert_eq!(max, v.last());
    log.save_svg("max_3.svg").expect("failed saving svg");
}
