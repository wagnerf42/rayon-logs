use rayon_logs::{subgraph, ThreadPoolBuilder};

fn invert(slice: &mut [u32]) {
    subgraph("invert slice", slice.len(), || {
        if slice.len() < 30_000 {
            (0..slice.len() / 2).for_each(|i| slice.swap(i, slice.len() - i - 1))
        } else {
            let (left, right) = slice.split_at_mut(slice.len() / 2);
            rayon_logs::join(|| invert(left), || invert(right));
        }
    })
}

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("failed creating pool");
    pool.install(|| {
        let mut v: Vec<u32> = subgraph("vector creation", 100_000, || (0..100_000).collect());
        invert(&mut v);
        assert_eq!(v[49_999], 25_000);
        assert_eq!(v[50_000], 74_999);
    });
}
