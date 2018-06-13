extern crate rayon;
extern crate rayon_logs;
use rayon_logs::Logger;

fn manual_max(logger: &Logger, slice: &[u32]) -> u32 {
    if slice.len() < 1000 {
        slice.iter().max().cloned().unwrap()
    } else {
        let middle = slice.len() / 2;
        let (left, right) = slice.split_at(middle);
        let (mleft, mright) =
            logger.join(|| manual_max(logger, left), || manual_max(logger, right));
        std::cmp::max(mleft, mright)
    }
}

fn main() {
    let v: Vec<u32> = (0..5000).collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("building pool failed");
    let logger = Logger::new(&pool);
    let max = pool.install(|| manual_max(&logger, &v));
    assert_eq!(max, v.last().cloned().unwrap());
    logger
        .save_logs("manual_max.json")
        .expect("saving logs file failed");
}
