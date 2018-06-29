use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use LoggedPool;

/// Builder for LoggedPool
#[derive(Default)]
pub struct LoggedPoolBuilder {
    real_builder: ThreadPoolBuilder,
}

impl LoggedPoolBuilder {
    /// Creates a new LoggedPoolBuilder.
    pub fn new() -> Self {
        LoggedPoolBuilder {
            real_builder: ThreadPoolBuilder::new(),
        }
    }

    /// Sets the number of threads to use.
    pub fn num_threads(self, num_threads: usize) -> Self {
        LoggedPoolBuilder {
            real_builder: self.real_builder.num_threads(num_threads),
        }
    }
    /// Build the `LoggedPool`.
    pub fn build(self) -> Result<LoggedPool, ThreadPoolBuildError> {
        self.real_builder.build().map(|p| LoggedPool::new(p))
    }
}
