extern crate rayon_logs;
use rayon_logs::{load_log_file, visualization, write_svg_file};
use std::env::args;

fn main() {
    let file = args()
        .nth(1)
        .expect("please, give a log file as first argument");
    let output_file = args()
        .nth(2)
        .expect("please, give a svg file name as second argument");
    let logs = load_log_file(&file).expect("failed reading log file");
    let (rectangles, edges) = visualization(logs.as_slice());
    write_svg_file(&rectangles, &edges, 1280, 1024, 10, output_file).expect("failed saving svg");
}
