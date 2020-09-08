//! Example for recursive max and tagging of leaves tasks.
use rayon_logs::{join, subgraph, Logger};

fn manual_max(slice: &[u32]) -> u32 {
    if slice.len() < 200_000 {
        subgraph("max", slice.len(), || slice.iter().max().cloned().unwrap())
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) = join(|| manual_max(left), || manual_max(right));
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let mut logger = Logger::new(); // we'll log the vector creation
    let v: Vec<u32> = (0..2_000_000).collect();

    let max = manual_max(&v);
    assert_eq!(max, v.last().cloned().unwrap());

    logger
        .save_raw_logs("manual_max.rlog")
        .expect("saving raw log file failed");
    println!("saved \"manual_max.rlog\"");
    println!("convert it using rlog2svg and open it in firefox");
}
