extern crate itertools;
extern crate rayon;
extern crate rayon_logs;

use rayon_logs::prelude::*;
use rayon_logs::ThreadPoolBuilder;
use std::collections::LinkedList;
use std::iter::once;

const LAST: u32 = 2_000_000;

fn main() {
    let v: Vec<u32> = (0..=LAST).collect();

    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .expect("building pool failed");
    pool.compare()
        .attach_algorithm("map_reduce", || {
            let f = v
                .par_iter()
                .filter(|&e| *e % 2 == 1)
                .map(|e| vec![*e])
                .reduce(Vec::new, |mut v1, v2| {
                    v1.extend(v2);
                    v1
                });
            assert_eq!(f.len() as u32, LAST / 2);
        })
        .attach_algorithm("fold_reduce", || {
            let f = v
                .par_iter()
                .filter(|&e| *e % 2 == 1)
                .fold(Vec::new, |mut v, e| {
                    v.push(*e);
                    v
                })
                .reduce(Vec::new, |mut v1, v2| {
                    v1.extend(v2);
                    v1
                });
            assert_eq!(f.len() as u32, LAST / 2);
        })
        .attach_algorithm("fold_map_reduce", || {
            let l = v
                .par_iter()
                .filter(|&e| *e % 2 == 1)
                .fold(Vec::new, |mut v, e| {
                    v.push(*e);
                    v
                })
                .map(|v| once(v).collect::<LinkedList<_>>())
                .reduce(LinkedList::new, |mut l1, mut l2| {
                    l1.append(&mut l2);
                    l1
                });
            let mut i = l.into_iter();
            let v = i.next().unwrap();
            let f = i.fold(v, |mut v, v2| {
                v.extend(v2);
                v
            });
            assert_eq!(f.len() as u32, LAST / 2);
        })
        .attach_algorithm_nodisplay("rayon", || {
            let f = v.par_iter().filter(|&e| *e % 2 == 1).collect::<Vec<_>>();
            assert_eq!(f.len() as u32, LAST / 2);
        })
        .generate_logs("filter.html")
        .expect("failed saving logs");
}
