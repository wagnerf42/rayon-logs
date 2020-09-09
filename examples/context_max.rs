//! Compute recursively a max using join_context.
use rayon::join_context;
use rayon_core::Logger;

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 1000 {
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
    let v: Vec<u32> = (0..10_000_000).collect();
    let mut logger = Logger::new();
    logger
        .pool_builder()
        .build_global()
        .expect("failed building pool");

    let max = manual_max(&v);
    assert_eq!(max, v.last().cloned().unwrap());

    logger
        .save_raw_logs("context_max.rlog")
        .expect("failed saving log");
}
