extern crate rayon_logs as rayon;
use rayon::{join_context, sequential_task, ThreadPoolBuilder};

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 1000 {
        sequential_task(0, slice.len(), || slice.iter().max().cloned().unwrap())
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join_context(
            |_| manual_max(left),
            |c| {
                if c.migrated() {
                    manual_max(right)
                } else {
                    sequential_task(0, right.len(), || *right.iter().max().unwrap())
                }
            },
        );
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..10_000_000).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");

    let (max, log) = pool.logging_install(|| manual_max(&v));
    assert_eq!(max, v.last().cloned().unwrap());

    log.save_svg("context_max.svg")
        .expect("saving svg file failed");
}
