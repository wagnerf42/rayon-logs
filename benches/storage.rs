#[macro_use]
extern crate criterion;
use criterion::{Criterion, ParameterizedBenchmark};
extern crate rayon_logs;
use rayon_logs::{RayonEvent, Storage};

fn storage(c: &mut Criterion) {
    let sizes = vec![1, 2, 3, 10, 100, 1000];
    c.bench(
        "store events",
        ParameterizedBenchmark::new(
            "store",
            |b, iterations| {
                b.iter_with_setup(
                    || Storage::new(),
                    |s| {
                        for _ in 0..*iterations {
                            s.push(RayonEvent::TaskStart(0, 0));
                        }
                    },
                )
            },
            sizes,
        ),
    );
}
criterion_group!(benches, storage);
criterion_main!(benches);
