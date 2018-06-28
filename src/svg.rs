//! Small module with display related functions.

use std::fs::File;
use std::io::prelude::*;
use std::io::Error;

pub(crate) type Point = (f64, f64);

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
    /// when animation starts and ends (if any)
    pub animation: Option<(u64, u64)>,
}

impl Rectangle {
    /// Creates a new rectangle
    pub fn new(
        color: [f32; 4],
        position: (f64, f64),
        sizes: (f64, f64),
        animation: Option<(u64, u64)>,
    ) -> Rectangle {
        Rectangle {
            color,
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
pub fn write_svg_file(
    rectangles: &[Rectangle],
    edges: &[(Point, Point)],
    svg_width: u32,
    svg_height: u32,
    duration: u32,
    path: &str,
) -> Result<(), Error> {
    let mut file = File::create(path)?;

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

    // we start by edges so they will end up below tasks
    for (start, end) in edges {
        file.write_fmt(format_args!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"0.01\"/>",
            start.0, start.1, end.0, end.1
        ))?;
    }

    for rectangle in rectangles {
        if let Some((start_time, end_time)) = rectangle.animation {
            file.write_fmt(
format_args!(
            "<rect x=\"{}\" y=\"{}\" width=\"0\" height=\"{}\" fill=\"rgba({},{},{},{})\">
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}s\" dur=\"{}s\" fill=\"freeze\"/>
</rect>\n",
        rectangle.x,
        rectangle.y,
        rectangle.height,
        (rectangle.color[0] * 255.0) as u32,
        (rectangle.color[1] * 255.0) as u32,
        (rectangle.color[2] * 255.0) as u32,
        rectangle.color[3],
        rectangle.width,
        (start_time *duration as u64) as f64 / last_time as f64,
        ((end_time - start_time) *duration as u64) as f64 / last_time as f64,
        )
        )?;
        } else {
            file.write_fmt(format_args!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\"/>\n",
                rectangle.x,
                rectangle.y,
                rectangle.width,
                rectangle.height,
                (rectangle.color[0] * 255.0) as u32,
                (rectangle.color[1] * 255.0) as u32,
                (rectangle.color[2] * 255.0) as u32,
                rectangle.color[3],
            ))?;
        }
    }
    file.write_all(b"</svg>")?;
    Ok(())
}
