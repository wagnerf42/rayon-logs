//! Small module with display related functions.

use std::fs::File;
use std::io::prelude::*;
use std::io::Error;

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

    let xscale = f64::from(svg_width) / (xmax - xmin);
    let yscale = f64::from(svg_height) / (ymax - ymin);

    // Header
    file.write_fmt(format_args!(
        "<?xml version=\"1.0\"?>
<svg width=\"{}\" height=\"{}\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        svg_width, svg_height,
    ))?;

    // we start by edges so they will end up below tasks
    for (start, end) in edges {
        file.write_fmt(format_args!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"2.0\"/>",
            (start.0 - xmin) * xscale,
            (start.1 - ymin) * yscale,
            (end.0 - xmin) * xscale,
            (end.1 - xmin) * yscale
        ))?;
    }

    for rectangle in rectangles {
        if let Some((start_time, end_time)) = rectangle.animation {
            file.write_fmt(
format_args!(
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
        )
        )?;
        } else {
            file.write_fmt(format_args!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\"/>\n",
                (rectangle.x - xmin) * xscale,
                (rectangle.y - ymin) * yscale,
                rectangle.width * xscale,
                rectangle.height * yscale,
                (rectangle.color[0] * 255.0) as u32,
                (rectangle.color[1] * 255.0) as u32,
                (rectangle.color[2] * 255.0) as u32,
                rectangle.opacity,
            ))?;
        }
    }
    file.write_all(b"</svg>")?;
    Ok(())
}
