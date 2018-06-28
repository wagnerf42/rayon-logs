extern crate rayon_logs;
use rayon_logs::{load_log_file, visualisation, write_svg_file};
use std::env::args;

fn main() {
    // we can specity the output file's name with -o
    let mut parsing_o = false;
    let mut input_files = Vec::new();
    let mut output_file = "output.svg".to_owned();

    for arg in args().skip(1) {
        if arg == "-h" || arg == "--help" {
            eprintln!("rayon2svg [-o output filename.svg] log1.json log2.json log3.json");
            return;
        }
        if arg == "-o" || arg == "--output" {
            parsing_o = true;
        } else {
            if parsing_o {
                output_file = arg.to_owned();
                parsing_o = false;
            } else {
                input_files.push(arg.to_owned());
            }
        }
    }

    // load all files
    let logs: Vec<_> = input_files
        .iter()
        .map(|filename| load_log_file(filename).expect(&format!("failed loading {}", filename)))
        .collect();

    // display all logs together
    let (rectangles, edges) = visualisation(&logs);
    write_svg_file(&rectangles, &edges, 1280, 1024, 10, &output_file).expect("failed saving svg");
}
