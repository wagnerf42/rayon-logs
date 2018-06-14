use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use LoggedPool;

/// Builder for LoggedPool
#[derive(Default)]
pub struct LoggedPoolBuilder {
    real_builder: ThreadPoolBuilder,
    filename: Option<String>,
}

impl LoggedPoolBuilder {
    /// Creates a new LoggedPoolBuilder.
    pub fn new() -> Self {
        LoggedPoolBuilder {
            real_builder: ThreadPoolBuilder::new(),
            filename: None,
        }
    }
    /// Specify a file to automatically save all logs when Pool will be dropped.
    pub fn log_file<S: Into<String>>(self, filename: S) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder,
            filename: Some(filename.into()),
        }
    }
    /// Sets the number of threads to use.
    pub fn num_threads(self, num_threads: usize) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder.num_threads(num_threads),
            filename: self.filename,
        }
    }
    /// Build the `LoggedPool`.
    pub fn build(self) -> Result<LoggedPool, ThreadPoolBuildError> {
        let filename = self.filename;
        self.real_builder
            .build()
            .map(|p| LoggedPool::new(p, filename))
    }
}
