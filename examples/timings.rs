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
    .take(10_000)
    .collect();
    eprintln!("it took {}", t.iter().map(|(t, _)| t).sum::<u64>());
    eprintln!("it took {}", t.iter().map(|(_, d)| d).sum::<u64>());
}
