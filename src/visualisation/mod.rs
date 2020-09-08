//! All code related to fork-join graphs and graphical display.
mod fork_join_graph;
pub(crate) use fork_join_graph::visualisation;
pub(crate) mod svg;
pub(crate) use svg::{fill_svg_file, histogram, write_svg_file, HISTOGRAM_COLORS};
