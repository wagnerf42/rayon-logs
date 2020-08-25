use crate::ThreadPool;
use rayon::{self, ThreadPoolBuildError};
type Builder = rayon::ThreadPoolBuilder;

/// We rewrite ThreadPoolBuilders since we need to overload the start handler
/// in order to give each thread a place to write its logs.
#[derive(Default)]
pub struct ThreadPoolBuilder {
    builder: Builder,
}

impl ThreadPoolBuilder {
    /// Creates a new ThreadPoolBuilder.
    pub fn new() -> Self {
        ThreadPoolBuilder {
            builder: Builder::new(),
        }
    }

    /// Set number of threads wanted.
    pub fn num_threads(self, threads_number: usize) -> Self {
        ThreadPoolBuilder {
            builder: self.builder.num_threads(threads_number),
        }
    }

    /// Build the `ThreadPool`.
    pub fn build(self) -> Result<ThreadPool, ThreadPoolBuildError> {
        let pool = self.builder.build();

        pool.map(|p| ThreadPool { pool: p })
    }
}
