//extern crate rayon_adaptive;
extern crate rayon_logs;
use rayon_logs::{make_subgraph, prelude::*, RunLog, ThreadPoolBuilder};

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .expect("Thread pool creation failed");
    let log = pool
        .logging_install(|| {
            (0..10).into_par_iter().for_each(|num| {
                make_subgraph("second level", 100, || {
                    (0..100).into_par_iter().for_each(|idk| {
                        //(0..100).into_par_iter().for_each(|idk1| {
                        assert!(idk + num >= 0);
                        //});
                    });
                });
            })
        })
        .1;
    log.save_svg("fullsvg.svg");
}
