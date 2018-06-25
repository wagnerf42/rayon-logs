//! Small module with display related functions.

use std::fs::File;
use std::io::prelude::*;
use std::io::Error;

/// colors used for each thread
pub(crate) const COLORS: [[f32; 4]; 8] = [
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [0.5, 0.5, 0.5, 1.0],
    [1.0, 0.5, 0.5, 1.0],
];

/// Tasks are animated as a set of rectangles.
pub struct Rectangle {
    /// color (rgb+alpha)
    pub color: [f32; 4],
    /// x coordinate
    pub x: f64,
    /// y coordinate
    pub y: f64,
    /// width
    pub width: f64,
    /// height
    pub height: f64,
    /// when animation starts (width gradually increases from 0 to real width)
    pub start_time: u64,
    /// when animation ends
    pub end_time: u64,
}

impl Rectangle {
    /// Creates a new rectangle
    pub fn new(
        color: [f32; 4],
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        start_time: u64,
        end_time: u64,
    ) -> Rectangle {
        Rectangle {
            color,
            x,
            y,
            width,
            height,
            start_time,
            end_time,
        }
    }
}

/// saves a set of rectangles as an animated svg file.
/// duration is the total duration of the animation in seconds.
pub fn write_svg_file(
    rectangles: &[Rectangle],
    svg_width: u64,
    svg_height: u64,
    duration: u64,
    path: String,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    let last_time = rectangles
        .iter()
        .map(|r| r.end_time)
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

    // Header
    file.write_fmt(format_args!(
        "<?xml version=\"1.0\"?>
<svg width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" preserveAspectRatio=\"none\"
version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        svg_width,
        svg_height,
        xmin,
        ymin,
        xmax - xmin,
        ymax - ymin,
    ))?;

    for rec in rectangles {
        file.write_fmt(
format_args!(
            "<rect x=\"{}\" y=\"{}\" width=\"0\" height=\"{}\" fill=\"rgba({},{},{},{})\">
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}s\" dur=\"{}s\" fill=\"freeze\"/>
</rect>\n",
        rec.x,
        rec.y,
        rec.height,                                  // height
        (rec.color[0] * 255.0) as u32,               // R
        (rec.color[1] * 255.0) as u32,               // G
        (rec.color[2] * 255.0) as u32,               // B
        rec.color[3],                                // alpha
        rec.width,
        rec.start_time *duration / last_time,
        (rec.end_time - rec.start_time) *duration / last_time,  // dur
        )
        )?;
    }
    file.write_all(b"</svg>")?;
    Ok(())
}
