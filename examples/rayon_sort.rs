//! This example traces one of rayon's internal algorithm.
use rand::{prelude::SliceRandom, thread_rng};
use rayon_logs::prelude::*;
use rayon_logs::save_svg;
use rayon_logs::ThreadPoolBuilder;

fn main() {
    let mut ra = thread_rng();
    let mut v: Vec<u32> = (0..100_000).collect();
    let answer = v.clone();
    v.shuffle(&mut ra);

    let p = ThreadPoolBuilder::new().build().expect("builder failed");
    p.install(|| v.par_sort());
    assert_eq!(v, answer);
    save_svg("rayon_sort.svg").expect("saving svg file failed");
}
