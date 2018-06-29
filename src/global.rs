//! provide a global pool to avoid passing pool around.
use rayon::FnContext;
use std::io;
use std::path::Path;
use {LoggedPool, LoggedPoolBuilder};

lazy_static! {
    static ref POOL: LoggedPool = {
        LoggedPoolBuilder::new()
            .build()
            .expect("building global pool failed")
    };
}

/// Execute a logging join_context.
pub fn join_context<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce(FnContext) -> RA + Send,
    B: FnOnce(FnContext) -> RB + Send,
    RA: Send,
    RB: Send,
{
    POOL.join_context(oper_a, oper_b)
}

/// Execute a logging join.
pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    POOL.join(oper_a, oper_b)
}

/// Execute given closure in the global thread pool, logging it's task as the initial one.
pub fn install<OP, R>(op: OP) -> R
where
    OP: FnOnce() -> R + Send,
    R: Send,
{
    POOL.install(op)
}

/// save tasks logs in json file.
/// DO NOT USE WHEN COMPUTATIONS ARE RUNNING.
pub fn save_logs<P: AsRef<Path>>(path: P) -> Result<(), io::Error> {
    POOL.save_logs(path)
}

/// Save an svg file of all logged information.
/// DO NOT USE WHEN COMPUTATIONS ARE RUNNING
pub fn save_svg<P: AsRef<Path>>(
    width: u32,
    height: u32,
    duration: u32,
    path: P,
) -> Result<(), io::Error> {
    POOL.save_svg(width, height, duration, path)
}
