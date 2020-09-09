//! This is rayon's scope documentation example.
//! Log will be saved as "log_0.json".

fn main() {
    let mut logger = rayon_core::Logger::new();
    // point start
    rayon::scope(|s| {
        s.spawn(|s| {
            // task s.1
            s.spawn(|_s| {
                // task s.1.1
                rayon::scope(|t| {
                    t.spawn(|_| ()); // task t.1
                    t.spawn(|_| ()); // task t.2
                });
            });
        });
        s.spawn(|_s| {
            // task 2
        });
        // point mid
    });
    // point end
    logger
        .save_raw_logs("scope.rlog")
        .expect("error saving svg");
}
