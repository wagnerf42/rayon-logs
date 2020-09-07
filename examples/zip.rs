//! This is an example about how to work around current limitations.
use rayon::prelude::*;
use rayon_logs::Logged;

fn main() {
    assert!(Logged::new(
        (0..10_000)
            .into_par_iter()
            .zip((0..10_000).into_par_iter().rev())
            .map(|(a, b)| a + b)
    )
    .all(|x| x == 9_999));
    rayon_logs::save_svg("zip.svg").expect("error saving svg");
}
