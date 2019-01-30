//! This is rayon's scope documentation example.
extern crate rayon_logs as rayon;
use rayon::{scope, ThreadPoolBuilder};

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let (_, log) = pool.install(|| {
        // point start
        rayon::scope(|s| {
            s.spawn(|s| {
                // task s.1
                s.spawn(|s| {
                    // task s.1.1
                    rayon::scope(|t| {
                        t.spawn(|_| ()); // task t.1
                        t.spawn(|_| ()); // task t.2
                    });
                });
            });
            s.spawn(|s| {
                // task 2
            });
            // point mid
        });
        // point end
    });

    log.save_svg("scope.svg").expect("saving svg file failed");
}
