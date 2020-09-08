//! Basic iterator log.
extern crate rayon_logs as rayon;
use rayon::Logger;

fn main() {
    let mut logger = Logger::new();
    let (s1, s2) = rayon::join(
        || (0..10_000_000).sum::<u64>(),
        || (0..20_000_000).sum::<u64>(),
    );
    assert!(s1 < s2);
    logger
        .save_raw_logs("join.rlog")
        .expect("failed saving log");
}
