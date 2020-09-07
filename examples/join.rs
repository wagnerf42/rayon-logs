//! Basic iterator log.
extern crate rayon_logs as rayon;
use rayon::save_svg;

fn main() {
    let (s1, s2) = rayon::join(
        || (0..10_000_000).sum::<u64>(),
        || (0..20_000_000).sum::<u64>(),
    );
    assert!(s1 < s2);
    save_svg("join.svg").expect("failed saving svg");
}
