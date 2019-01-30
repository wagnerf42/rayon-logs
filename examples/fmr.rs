//! just a small example of a complex operation using fold
extern crate rayon_logs as rayon;
use rayon::{prelude::*, sequential_task, ThreadPoolBuilder};
use std::collections::LinkedList;

fn main() {
    let v: Vec<u32> = (0..2_000_000).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");

    let (even_elements, log) = pool.logging_install(|| {
        let mut vecs = v
            .par_iter()
            .filter(|&e| *e % 2 == 0)
            .fold(Vec::new, |mut v, e| {
                v.push(*e);
                v
            })
            .map(|v| {
                let mut l = LinkedList::new();
                l.push_back(v);
                l
            })
            .reduce(LinkedList::new, |mut l1, mut l2| {
                l1.append(&mut l2);
                l1
            })
            .into_iter();
        let final_vec = vecs.next().unwrap();
        vecs.fold(final_vec, |mut f, v| {
            sequential_task(3, v.len(), || {
                f.extend(v);
                f
            })
        })
    });

    assert_eq!(even_elements.len(), 1_000_000);

    log.save_svg("fold_map_reduce.svg")
        .expect("saving svg file failed");
}
