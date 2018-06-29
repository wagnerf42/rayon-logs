//! provide a global pool to avoid passing pool around.
use rayon::FnContext;
use {LoggedPool, LoggedPoolBuilder, RunLog};

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
pub fn install<OP, R>(op: OP) -> (R, RunLog)
where
    OP: FnOnce() -> R + Send,
    R: Send,
{
    POOL.install(op)
}

/// Tag currently active task with a type and amount of work.
pub fn log_work(work_type: usize, work_amount: usize) {
    POOL.log_work(work_type, work_amount)
}
