//! this example traces one of rayon's internal algorithm.
extern crate rand;
extern crate rayon_logs as rayon;
use rand::{ChaChaRng, Rng};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

fn main() {
    let mut ra = ChaChaRng::new_unseeded();
    let mut v: Vec<u32> = (0..100_000).collect();
    let answer = v.clone();
    ra.shuffle(&mut v);

    let p = ThreadPoolBuilder::new().build().expect("builder failed");
    let log = p.install(|| v.par_sort()).1;
    log.save_svg("rayon_sort.svg")
        .expect("saving svg file failed");
}
