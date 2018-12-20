extern crate rayon_logs as rayon;
use rayon::{prelude::*, ThreadPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..10_000_000).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (first, log) = pool.install(|| v.par_iter().find_first(|&x| *x == 4_800_000).cloned());
    assert_eq!(first, Some(4_800_000));

    log.save_svg("find_first.svg")
        .expect("saving svg file failed");
}
