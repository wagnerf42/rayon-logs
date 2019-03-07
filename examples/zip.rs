//! This is an example about how to work around current limitations.
use rayon::prelude::*;
use rayon_logs::Logged;
use rayon_logs::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .build()
        .expect("failed creating thread pool");
    assert!(pool.install(|| Logged::new(
        (0..10_000)
            .into_par_iter()
            .zip((0..10_000).into_par_iter().rev())
            .map(|(a, b)| a + b)
    )
    .all(|x| x == 9_999)));
}
