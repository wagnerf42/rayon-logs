extern crate rayon_logs;
use rayon_logs::{prelude::*, LoggedPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..1_000_000).collect();
    let pool = LoggedPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");

    let (max, log) = pool.install(|| v.par_iter().log(&pool).max().cloned().unwrap());
    assert_eq!(max, v.last().cloned().unwrap());
    log.save_svg(1920, 1080, 20, "max.svg")
        .expect("saving svg failed");
}
