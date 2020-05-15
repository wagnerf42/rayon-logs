//! Compare several filter collect algorithms and generate an html comparison page.
use rayon::prelude::*;
use rayon_logs::{subgraph, Logged, ThreadPoolBuilder};

const SIZE: usize = 20_000_000;

fn seq_prefixe(t: &mut [u64]) -> u64 {
    subgraph("prefixe seq", t.len(), || {
        t.iter_mut().fold(0, |s, e| {
            *e += s;
            *e
        })
    })
}

fn prefixe(t: &mut [u64]) {
    let c = (t.len() as f64).sqrt().ceil() as usize;
    let mut v: Vec<u64> = Logged::new(t.par_chunks_mut(c).map(seq_prefixe)).collect();
    seq_prefixe(&mut v);
    Logged::new(t.par_chunks_mut(c).skip(1).zip(v.par_iter()))
        .for_each(|(s, l)| subgraph("update", s.len(), || s.iter_mut().for_each(|e| *e += *l)));
}

fn prefixe2(t: &mut [u64]) {
    let c = t.len() / 2;
    let mut v: Vec<u64> = Logged::new(t.par_chunks_mut(c).map(seq_prefixe)).collect();
    seq_prefixe(&mut v);
    Logged::new(t.par_chunks_mut(c).skip(1).zip(v.par_iter()))
        .for_each(|(s, l)| subgraph("update", s.len(), || s.iter_mut().for_each(|e| *e += *l)));
}

fn prefixe_tres_par(t: &mut [u64]) -> u64 {
    if t.len() < 1_000 {
        seq_prefixe(t)
    } else {
        let c = (t.len() as f64).sqrt().ceil() as usize;
        let mut v: Vec<u64> = Logged::new(t.par_chunks_mut(c).map(prefixe_tres_par)).collect();
        let r = prefixe_tres_par(&mut v);
        Logged::new(t.par_chunks_mut(c).skip(1).zip(v.par_iter())).for_each(|(s, l)| {
            subgraph("update", s.len(), || {
                Logged::new(s.par_iter_mut()).for_each(|e| *e += *l)
            })
        }); // too many // computations here
        r
    }
}

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .expect("building pool failed");
    pool.compare()
        .runs_number(3)
        .attach_algorithm_nodisplay_with_setup(
            "seq",
            || std::iter::repeat(1).take(SIZE).collect::<Vec<u64>>(),
            |mut v| {
                seq_prefixe(&mut v);
                // assert!(v.iter().copied().eq(1..=SIZE as u64));
                v
            },
        )
        .attach_algorithm_nodisplay_with_setup(
            "prefixe par",
            || std::iter::repeat(1).take(SIZE).collect::<Vec<u64>>(),
            |mut v| {
                prefixe(&mut v);
                // assert!(v.iter().copied().eq(1..=SIZE as u64));
                v
            },
        )
        .attach_algorithm_nodisplay_with_setup(
            "prefixe par 2",
            || std::iter::repeat(1).take(SIZE).collect::<Vec<u64>>(),
            |mut v| {
                prefixe2(&mut v);
                // assert!(v.iter().copied().eq(1..=SIZE as u64));
                v
            },
        )
        .attach_algorithm_nodisplay_with_setup(
            "prefixe ultra par",
            || std::iter::repeat(1).take(SIZE).collect::<Vec<u64>>(),
            |mut v| {
                prefixe_tres_par(&mut v);
                // assert!(v.iter().copied().eq(1..=SIZE as u64));
                v
            },
        )
        .generate_logs("compare.html")
        .expect("failed saving logs");
    println!("generated compare.html");
}
