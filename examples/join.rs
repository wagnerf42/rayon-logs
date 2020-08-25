//! Basic iterator log.
extern crate rayon_logs as rayon;
use rayon::{prelude::*, save_svg, ThreadPoolBuilder};

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (s1, s2) = pool.install(|| {
        rayon::join(
            || (0..10_000_000).sum::<u64>(),
            || (0..20_000_000).sum::<u64>(),
        )
    });
    assert!(s1 < s2);
    save_svg("join.svg").expect("failed saving svg");
}
