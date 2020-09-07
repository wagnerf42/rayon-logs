use rayon_logs::log2svg;
use std::env::args;

fn main() {
    let log_file = args()
        .nth(1)
        .expect("missing rlog file name as first argument");
    let svg_file = args()
        .nth(2)
        .expect("missing svg file name as second argument");
    log2svg(&log_file, &svg_file).expect("io error");
}
