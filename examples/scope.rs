//! This is rayon's scope documentation example.
//! Log will be saved as "log_0.json".
extern crate rayon_logs as rayon;

fn main() {
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
    rayon_logs::save_svg("scope.svg").expect("error saving svg");
}
