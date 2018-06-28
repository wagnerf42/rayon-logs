extern crate rayon_logs;
use rayon_logs::{prelude::*, LoggedPoolBuilder};

fn main() {
    let v: Vec<u32> = (0..1_000_000).collect();
    let pool = LoggedPoolBuilder::new()
        .num_threads(2)
        .log_file("max.json")
        .svg(1280, 1024, 10, "max.svg") // 1280x1024 ; 10 seconds animation
        .build()
        .expect("building pool failed");

    let max = pool.install(|| v.par_iter().log(&pool).max())
        .cloned()
        .unwrap();
    assert_eq!(max, v.last().cloned().unwrap());
}
