extern crate rayon_logs;
use rayon_logs::{join_context, log_work, ThreadPoolBuilder};

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 1000 {
        log_work(0, slice.len());
        slice.iter().max().cloned().unwrap()
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join_context(
            |_| manual_max(left),
            |c| {
                if c.migrated() {
                    manual_max(right)
                } else {
                    *right.iter().max().unwrap()
                }
            },
        );
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..1_000_000).collect();
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (max, log) = pool.install(|| manual_max(&v));
    assert_eq!(max, v.last().cloned().unwrap());

    log.save_svg(1920, 1080, 10, "context_max.svg")
        .expect("saving svg file failed");
}
