    extern crate rayon_logs;

    use rayon_logs::RunLog;
    use std::env::args;

    fn main() {
        let json_file = args().nth(1).expect("missing json file name as first argument");
        let svg_file = args().nth(2).expect("missing svg file name as second argument");
        let logs = RunLog::load(&json_file).expect("failed to load json file");
        logs.save_svg(&svg_file).expect("failed to save svg file");
    }

