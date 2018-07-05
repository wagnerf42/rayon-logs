extern crate rayon_logs;
use rayon_logs::{join, sequential_task, ThreadPoolBuilder};

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 200_000 {
        sequential_task(0, slice.len(), || slice.iter().max().cloned().unwrap())
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join(|| manual_max(left), || manual_max(right));
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (max, log) = pool.install(|| manual_max(&v));
    assert_eq!(max, v.last().cloned().unwrap());

    log.save_svg("manual_max.svg")
        .expect("saving svg file failed");
}
