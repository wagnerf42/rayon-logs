extern crate rayon_logs;
use rayon_logs::{install, join, log_work};

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 200_000 {
        log_work(0, slice.len());
        slice.iter().max().cloned().unwrap()
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join(|| manual_max(left), || manual_max(right));
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();
    let (max, log) = install(|| manual_max(&v));
    log.save_svg(1920, 1080, 10, "manual_max.svg")
        .expect("saving svg file failed");
    assert_eq!(max, v.last().cloned().unwrap());
}
