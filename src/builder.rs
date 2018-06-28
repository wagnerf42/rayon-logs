use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use LoggedPool;

/// Builder for LoggedPool
#[derive(Default)]
pub struct LoggedPoolBuilder {
    real_builder: ThreadPoolBuilder,
    filename: Option<String>,
    svg: Option<(u32, u32, u32, String)>,
}

impl LoggedPoolBuilder {
    /// Creates a new LoggedPoolBuilder.
    pub fn new() -> Self {
        LoggedPoolBuilder {
            real_builder: ThreadPoolBuilder::new(),
            filename: None,
            svg: None,
        }
    }
    /// Specify a file to automatically save all logs when Pool will be dropped.
    pub fn log_file<S: Into<String>>(self, filename: S) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder,
            filename: Some(filename.into()),
            svg: self.svg,
        }
    }

    /// Specify a file to automatically create an svg animation when Pool will be dropped.
    /// also takes width and height of svg and animation duration (in seconds).
    pub fn svg<S: Into<String>>(self, width: u32, height: u32, duration: u32, filename: S) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder,
            filename: self.filename,
            svg: Some((width, height, duration, filename.into())),
        }
    }

    /// Sets the number of threads to use.
    pub fn num_threads(self, num_threads: usize) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder.num_threads(num_threads),
            filename: self.filename,
            svg: self.svg,
        }
    }
    /// Build the `LoggedPool`.
    pub fn build(self) -> Result<LoggedPool, ThreadPoolBuildError> {
        let filename = self.filename;
        let svg_parameters = self.svg;
        self.real_builder
            .build()
            .map(|p| LoggedPool::new(p, filename, svg_parameters))
    }
}
