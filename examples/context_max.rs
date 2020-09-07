//! Compute recursively a max using join_context.
extern crate rayon_logs as rayon; // comment me out to go back to rayon
use rayon::join_context;

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 1000 {
        slice.iter().max().cloned().unwrap()
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join_context(
            |_| manual_max(left),
            |c| {
                if c.migrated() {
                    manual_max(right)
                } else {
                    *right.iter().max().unwrap()
                }
            },
        );
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..10_000_000).collect();

    let max = manual_max(&v);
    assert_eq!(max, v.last().cloned().unwrap());

    rayon_logs::save_svg("context_max.svg").expect("failed saving svg");
}
