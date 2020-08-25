use rayon::prelude::*;
use rayon_logs::{join, save_svg, subgraph, Logged, ThreadPoolBuilder};
use std::thread::sleep_ms;

fn main() {
    let pool = ThreadPoolBuilder::new().build().expect("failed pool");
    pool.install(|| {
        Logged::new(
            (0..100u32)
                .into_par_iter()
                .map(|i| subgraph("outer stuff", i as usize, || (0..i).collect::<Vec<u32>>())),
        )
        .reduce(Vec::new, |mut a, mut b| {
            subgraph("reducy", b.len(), || {
                a.append(&mut b);
                a
            })
        })
    });
    save_svg("tags.svg").unwrap()
}
