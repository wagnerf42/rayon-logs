//! Logging scope and Scope.
use crate::{pool::log, pool::next_task_id, RayonEvent, TaskId};
use std::mem::transmute;
use time::precise_time_ns;

pub struct Scope<'scope> {
    rayon_scope: Option<&'scope rayon::Scope<'scope>>,
    continuing_task_id: TaskId,
}

impl<'scope> Scope<'scope> {
    pub fn spawn<BODY>(&self, body: BODY)
    where
        BODY: FnOnce(&Scope<'scope>) + Send + 'scope,
    {
        let spawned_id = next_task_id();
        log(RayonEvent::Child(spawned_id));
        // sorry I need to erase the borrow's lifetime.
        // it's ok though since the pointed self will survive all spawned tasks.
        let floating_self: &'scope Scope<'scope> = unsafe { transmute(self) };
        let logged_body = move |_: &rayon::Scope<'scope>| {
            log(RayonEvent::TaskStart(spawned_id, precise_time_ns()));
            body(floating_self);
            log(RayonEvent::Child(floating_self.continuing_task_id));
            log(RayonEvent::TaskEnd(precise_time_ns()));
        };
        self.rayon_scope.as_ref().unwrap().spawn(logged_body);
    }
}

/// Create a "fork-join" scope `s` and invokes the closure with a
/// reference to `s`. This closure can then spawn asynchronous tasks
/// into `s`. Those tasks may run asynchronously with respect to the
/// closure; they may themselves spawn additional tasks into `s`. When
/// the closure returns, it will block until all tasks that have been
/// spawned into `s` complete.
pub fn scope<'scope, OP, R>(op: OP) -> R
where
    OP: for<'s> FnOnce(&'s Scope<'scope>) -> R + 'scope + Send,
    R: Send,
{
    let scope_id = next_task_id();
    let continuing_task_id = next_task_id();
    log(RayonEvent::Child(scope_id));
    log(RayonEvent::TaskEnd(precise_time_ns()));
    // the Scope structure needs to survive the scope fn call
    // because tasks might be executed AFTER the op call completed
    let mut borrowed_scope: Scope<'scope> = Scope {
        rayon_scope: None, // we cannot know now so we use a None
        continuing_task_id,
    };
    let borrowed_scope_ref = &mut borrowed_scope;
    let r = rayon::scope(move |s| {
        log(RayonEvent::TaskStart(scope_id, precise_time_ns()));
        // I'm sorry, there is no other way to do it without changing
        // the API. Because I can only access a reference to the underlying rayon::Scope
        borrowed_scope_ref.rayon_scope = unsafe { transmute(Some(s)) };
        let r = op(borrowed_scope_ref);
        log(RayonEvent::Child(continuing_task_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        r
    });
    log(RayonEvent::TaskStart(continuing_task_id, precise_time_ns()));
    r
}
