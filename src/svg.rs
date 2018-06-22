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
