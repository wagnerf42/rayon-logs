extern crate rayon_logs;
use rayon_logs::{prelude::*, LoggedPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..5000).collect();
    let pool = LoggedPoolBuilder::new()
        .num_threads(2)
        .log_file("max.json")
        .build()
        .expect("building pool failed");

    let max = pool
        .install(|| v.par_iter().log(&pool).max())
        .cloned()
        .unwrap();
    assert_eq!(max, v.last().cloned().unwrap());
}
