//! Logging scope and Scope.
use crate::{pool::log, pool::next_task_id, RayonEvent, TaskId};
use time::precise_time_ns;

use rayon::Scope;

pub struct BorrowedScope<'scope, 'realscope> {
    rayon_scope: &'scope Scope<'realscope>,
    continuing_task_id: TaskId,
}

impl<'scope, 'realscope> BorrowedScope<'scope, 'realscope> {
    pub fn spawn<BODY>(&self, body: BODY)
    where
        BODY: FnOnce(&BorrowedScope<'scope, 'realscope>) + Send + 'realscope,
    {
        unimplemented!()
    }
}

/// Create a "fork-join" scope `s` and invokes the closure with a
/// reference to `s`. This closure can then spawn asynchronous tasks
/// into `s`. Those tasks may run asynchronously with respect to the
/// closure; they may themselves spawn additional tasks into `s`. When
/// the closure returns, it will block until all tasks that have been
/// spawned into `s` complete.
pub fn scope<'realscope, OP, R>(op: OP) -> R
where
    OP: for<'a, 'scope> FnOnce(&'a BorrowedScope<'scope, 'realscope>) -> R + 'realscope + Send,
    R: Send,
{
    let scope_id = next_task_id();
    let continuing_task_id = next_task_id();
    log(RayonEvent::Child(scope_id));
    log(RayonEvent::TaskEnd(precise_time_ns()));
    let r = rayon::scope(move |s| {
        log(RayonEvent::TaskStart(scope_id, precise_time_ns()));
        let borrowed_scope = BorrowedScope {
            rayon_scope: s,
            continuing_task_id,
        };
        let r = op(&borrowed_scope);
        log(RayonEvent::Child(continuing_task_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        r
    });
    log(RayonEvent::TaskStart(continuing_task_id, precise_time_ns()));
    r
}
