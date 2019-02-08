//extern crate rayon_adaptive;
extern crate rayon_logs;
use rayon_logs::{make_subgraph, prelude::*, ThreadPoolBuilder};

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool creation failed");
    let log = pool
        .logging_install(|| {
            (0..17).into_par_iter().for_each(|num| {
                make_subgraph("second level", 100, || {
                    (0..13).into_par_iter().for_each(|idk| {
                        make_subgraph("third level", 100, || {
                            (0..10).into_par_iter().for_each(|idk1| {
                                assert!(idk1 * num + idk >= 0);
                            });
                        });
                    });
                });
            })
        })
        .1;
    log.save_svg("fullsvg.svg");
    log.save("mylog.json");
}
