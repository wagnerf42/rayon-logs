//! this example traces one of rayon's internal algorithm.
extern crate rayon_logs as rayon;
use rand::{prelude::SliceRandom, thread_rng};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

fn main() {
    let mut ra = thread_rng();
    let mut v: Vec<u32> = (0..100_000).collect();
    let answer = v.clone();
    v.shuffle(&mut ra);

    let p = ThreadPoolBuilder::new().build().expect("builder failed");
    let log = p.logging_install(|| v.par_sort()).1;
    assert_eq!(v, answer);
    log.save_svg("rayon_sort.svg")
        .expect("saving svg file failed");
}
