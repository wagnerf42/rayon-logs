//! Small module with display related functions.

use itertools::repeat_call;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::iter::repeat;
use std::path::Path;
use RunLog;

pub(crate) type Point = (f64, f64);

/// colors used for each thread
pub(crate) const COLORS: [[f32; 3]; 8] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 1.0, 0.0],
    [1.0, 0.0, 1.0],
    [0.0, 1.0, 1.0],
    [0.5, 0.5, 0.5],
    [1.0, 0.5, 0.5],
];

/// Tasks are animated as a set of rectangles.
pub struct Rectangle {
    /// color (rgb+alpha)
    pub color: [f32; 3],
    /// opacity
    pub opacity: f32,
    /// x coordinate
    pub x: f64,
    /// y coordinate
    pub y: f64,
    /// width
    pub width: f64,
    /// height
    pub height: f64,
    /// when animation starts and ends (if any)
    pub animation: Option<(u64, u64)>,
}

impl Rectangle {
    /// Creates a new rectangle
    pub fn new(
        color: [f32; 3],
        opacity: f32,
        position: (f64, f64),
        sizes: (f64, f64),
        animation: Option<(u64, u64)>,
    ) -> Rectangle {
        Rectangle {
            color,
            opacity,
            x: position.0,
            y: position.1,
            width: sizes.0,
            height: sizes.1,
            animation,
        }
    }
}

/// saves a set of rectangles and edges as an animated svg file.
/// duration is the total duration of the animation in seconds.
pub fn write_svg_file<P: AsRef<Path>>(
    rectangles: &[Rectangle],
    edges: &[(Point, Point)],
    svg_width: u32,
    svg_height: u32,
    duration: u32,
    path: P,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    fill_svg_file(
        rectangles, edges, svg_width, svg_height, duration, &mut file,
    )
}

/// fill given file with a set of rectangles and edges as an animated svg.
/// duration is the total duration of the animation in seconds.
pub fn fill_svg_file(
    rectangles: &[Rectangle],
    edges: &[(Point, Point)],
    svg_width: u32,
    svg_height: u32,
    duration: u32,
    file: &mut File,
) -> Result<(), Error> {
    let last_time = rectangles
        .iter()
        .filter_map(|r| r.animation.map(|(_, end_time)| end_time))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let xmax = rectangles
        .iter()
        .map(|r| r.width + r.x)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let ymax = rectangles
        .iter()
        .map(|r| r.height + r.y)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let xmin = rectangles
        .iter()
        .map(|r| r.x)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let ymin = rectangles
        .iter()
        .map(|r| r.y)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let xscale = f64::from(svg_width) / (xmax - xmin);
    let yscale = f64::from(svg_height) / (ymax - ymin);

    // Header
    write!(
        file,
        "<?xml version=\"1.0\"?>
<svg viewBox=\"0 0 {} {}\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        svg_width, svg_height,
    )?;

    // we start by edges so they will end up below tasks
    for (start, end) in edges {
        write!(
            file,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"2.0\"/>",
            (start.0 - xmin) * xscale,
            (start.1 - ymin) * yscale,
            (end.0 - xmin) * xscale,
            (end.1 - xmin) * yscale
        )?;
    }

    for rectangle in rectangles {
        if let Some((start_time, end_time)) = rectangle.animation {
            write!(file,
            "<rect x=\"{}\" y=\"{}\" width=\"0\" height=\"{}\" fill=\"rgba({},{},{},{})\">
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}s\" dur=\"{}s\" fill=\"freeze\"/>
</rect>\n",
        (rectangle.x-xmin)*xscale,
        (rectangle.y-ymin)*yscale,
        rectangle.height*yscale,
        (rectangle.color[0] * 255.0) as u32,
        (rectangle.color[1] * 255.0) as u32,
        (rectangle.color[2] * 255.0) as u32,
        rectangle.opacity,
        rectangle.width*xscale,
        (start_time * u64::from(duration)) as f64 / last_time as f64,
        ((end_time - start_time) * u64::from(duration)) as f64 / last_time as f64,
        )?;
        } else {
            write!(
                file,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\"/>\n",
                (rectangle.x - xmin) * xscale,
                (rectangle.y - ymin) * yscale,
                rectangle.width * xscale,
                rectangle.height * yscale,
                (rectangle.color[0] * 255.0) as u32,
                (rectangle.color[1] * 255.0) as u32,
                (rectangle.color[2] * 255.0) as u32,
                rectangle.opacity,
            )?;
        }
    }
    write!(file, "</svg>")?;
    Ok(())
}

/// Display histogram for given logs set inside html file.
pub(crate) fn histogram(
    file: &mut File,
    logs: &[Vec<RunLog>],
    bars_number: usize,
) -> Result<(), Error> {
    let min_duration = logs.iter()
        .map(|l| l.first().map(|fl| fl.duration).unwrap())
        .min()
        .unwrap();
    let max_duration = logs.iter()
        .map(|l| l.last().map(|ll| ll.duration).unwrap())
        .max()
        .unwrap();

    // lets compute how many durations go in each bar
    let mut bars: Vec<Vec<usize>> = repeat_call(|| repeat(0).take(bars_number).collect())
        .take(logs.len())
        .collect();
    let slot = (max_duration - min_duration) / bars_number as u64;
    for (algorithm, algorithm_logs) in logs.iter().enumerate() {
        for duration in algorithm_logs.iter().map(|l| l.duration) {
            let mut index = ((duration - min_duration) / slot) as usize;
            if index == bars_number {
                index -= 1;
            }
            bars[algorithm][index] += 1;
        }
    }

    // now, just draw one rectangle for each bar
    let width = 1920;
    let height = 1080;
    write!(file, "<svg viewBox=\"0 0 {} {}\">", width, height)?;
    write!(
        file,
        "<rect width=\"{}\" height=\"{}\" fill=\"white\"/>",
        width, height
    )?;
    let max_count = bars.iter().flat_map(|b| b.iter()).max().unwrap();
    let unit_height = (height - 100) as f32 / *max_count as f32;
    let unit_width = width as f32 / (bars_number as f32 * 1.5);
    let colors = ["red", "blue"];
    for (algorithm_index, (counts, color)) in bars.iter().zip(colors.iter().cycle()).enumerate() {
        for (index, &count) in counts.iter().enumerate() {
            if count != 0 {
                write!(
                    file,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    algorithm_index as f32 * unit_width / 2.0 + unit_width * 1.5 * index as f32,
                    (height - 100) as f32 - (count as f32 * unit_height),
                    unit_width / 2.0,
                    count as f32 * unit_height,
                    color
                )?;
            }
        }
    }
    write!(
        file,
        "<text x=\"{}\" y=\"{}\">{}</text>",
        width / 2,
        height - 50,
        (min_duration + max_duration) / 2
    )?;
    write!(
        file,
        "<text x=\"{}\" y=\"{}\">{}</text>",
        100,
        height - 50,
        min_duration
    )?;
    write!(
        file,
        "<text x=\"{}\" y=\"{}\">{}</text>",
        width - 100,
        height - 50,
        max_duration
    )?;
    write!(file, "</svg>")?;
    Ok(())
}
