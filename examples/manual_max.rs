extern crate rayon_logs;
use rayon_logs::{LoggedPool, LoggedPoolBuilder};

fn manual_max(pool: &LoggedPool, slice: &[u32]) -> u32 {
    if slice.len() < 200_000 {
        pool.log_work(0, slice.len());
        slice.iter().max().cloned().unwrap()
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = pool.join(|| manual_max(pool, left), || manual_max(pool, right));
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();
    let pool = LoggedPoolBuilder::new()
        .num_threads(2)
        .log_file("manual_max.json")
        .svg(1920, 1080, 10, "manual_max.svg") // 1280x1024 ; 10 seconds animation
        .build()
        .expect("building pool failed");
    let max = pool.install(|| manual_max(&pool, &v));
    assert_eq!(max, v.last().cloned().unwrap());
}
