extern crate rayon;
extern crate rayon_logs;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use rayon_logs::LoggedPool;

fn main() {
    let v: Vec<u32> = (0..5000).collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let pool = LoggedPool::new(&pool);

    let max = pool
        .install(|| rayon_logs::iterator::new(v.par_iter()).max())
        .cloned()
        .unwrap();
    assert_eq!(max, v.last().cloned().unwrap());
    pool.save_logs("manual_max.json")
        .expect("saving logs file failed");
}
