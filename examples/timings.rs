//! This example exhibits logging overheads.
extern crate rayon_logs;
use rayon_logs::ThreadPoolBuilder;
use std::iter::repeat_with;

fn fibo(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibo(n - 1) + fibo(n - 2)
    }
}

const REPETITIONS: usize = 1_000_000;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("building pool failed");

    let t: Vec<(u64, u64)> = repeat_with(|| {
        let (t, d) = pool.logging_install(|| {
            let start = std::time::Instant::now();
            let x = fibo(10);
            assert!(x > 0);
            start.elapsed().as_nanos() as u64
        });
        (t, d.duration)
    })
    .take(REPETITIONS)
    .collect();

    let inner_time = t.iter().map(|(t, _)| t).sum::<u64>();
    let outer_time = t.iter().map(|(_, d)| d).sum::<u64>();
    let overhead = (outer_time - inner_time) / REPETITIONS as u64;
    println!(
        "we estimate a logging overhead of approximately {} ns per logged event",
        overhead
    );
    println!(
        "if you want to keep logging overheads below 1% we advise you to log
        only tasks with a duration larger than {} ns",
        overhead * 100
    );
}
