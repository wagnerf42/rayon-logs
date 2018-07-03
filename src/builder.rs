use pool::LOGS;
use rayon::{self, ThreadPoolBuildError};
use std::sync::{Arc, Mutex};
use storage::Storage;
use ThreadPool;

/// We rewrite ThreadPoolBuilders since we need to overload the start handler
/// in order to give each thread a place to write its logs.
#[derive(Default)]
pub struct ThreadPoolBuilder {
    builder: rayon::ThreadPoolBuilder,
}

impl ThreadPoolBuilder {
    /// Creates a new LoggedPoolBuilder.
    pub fn new() -> Self {
        ThreadPoolBuilder {
            builder: rayon::ThreadPoolBuilder::new(),
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
        let logs = Arc::new(Mutex::new(Vec::new()));
        let shared_logs = logs.clone();

        let pool = self.builder
            .start_handler(move |_| {
                LOGS.with(|l| {
                    let thread_storage = Arc::new(Storage::new());
                    shared_logs.lock().unwrap().push(thread_storage.clone());
                    *l.borrow_mut() = thread_storage;
                });
            })
            .build();

        pool.map(|p| ThreadPool { pool: p, logs })
    }
}
