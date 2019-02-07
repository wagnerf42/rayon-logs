extern crate rayon_logs;
use rayon_logs::prelude::*;
use rayon_logs::ThreadPoolBuilder;
fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .expect("building pool failed");
    pool.logging_install(|| {
        (0..100_000)
            .into_par_iter()
            //.zip((0..10_000).into_par_iter())
            //.filter(|num| num % 5 == 0)
            .fold(Vec::new, |mut v, num| {
                v.push(num);
                v
            })
            .map(|v| v.par_iter().sum::<i32>())
            .reduce(|| 0, |a, b| a + b);
    })
    .1
    .save_svg("collect_test.html")
    .expect("SVG creation failed");
}
