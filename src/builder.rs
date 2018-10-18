use pool::LOGS;
extern crate hwloc;
extern crate libc;
use self::hwloc::{CpuSet, ObjectType, Topology, CPUBIND_THREAD};
use rayon::{self, ThreadPoolBuildError};
use std::sync::{Arc, Mutex};
use storage::Storage;
use ThreadPool;

/// We rewrite ThreadPoolBuilders since we need to overload the start handler
/// in order to give each thread a place to write its logs.
#[derive(Default)]
pub struct ThreadPoolBuilder {
    builder: rayon::ThreadPoolBuilder,
    bind_to_core: bool,
}

fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!(
            "I won't allow you to have {} more threads than logical cores!",
            idx - cores.len() + 1
        ),
    }
}

fn get_thread_id() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}

impl ThreadPoolBuilder {
    /// Creates a new LoggedPoolBuilder.
    pub fn new() -> Self {
        ThreadPoolBuilder {
            builder: rayon::ThreadPoolBuilder::new(),
            bind_to_core: false,
        }
    }

    /// Set number of threads wanted.
    pub fn num_threads(self, threads_number: usize) -> Self {
        ThreadPoolBuilder {
            builder: self.builder.num_threads(threads_number),
            bind_to_core: self.bind_to_core,
        }
    }

    /// Just call this method to make sure that the threads are bound to cores.
    pub fn bind_threads(self) -> Self {
        ThreadPoolBuilder {
            builder: self.builder,
            bind_to_core: true,
        }
    }

    /// Build the `ThreadPool`.
    pub fn build(self) -> Result<ThreadPool, ThreadPoolBuildError> {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let shared_logs = logs.clone();
        let topo = Mutex::new(Topology::new());
        let bind = self.bind_to_core;
        let pool = self
            .builder
            .start_handler(move |thread_id| {
                LOGS.with(|l| {
                    let thread_storage = Arc::new(Storage::new());
                    shared_logs.lock().unwrap().push(thread_storage.clone());
                    *l.borrow_mut() = thread_storage;
                });
                if bind {
                    binder(thread_id, &topo);
                }
            }).build();

        pool.map(|p| ThreadPool { pool: p, logs })
    }
}

fn binder(thread_id: usize, topo: &Mutex<Topology>) {
    let pthread_id = get_thread_id();
    let mut locked_topo = topo.lock().unwrap();
    let mut bind_to = cpuset_for_core(&locked_topo, thread_id);
    bind_to.singlify();
    println!("binding {} to {}", pthread_id, bind_to);
    locked_topo
        .set_cpubind_for_thread(pthread_id, bind_to, CPUBIND_THREAD)
        .unwrap();
    println!("binding done");
    let after = locked_topo.get_cpubind_for_thread(pthread_id, CPUBIND_THREAD);
    println!("Thread {}, bind to {:?}", thread_id, after);
}
