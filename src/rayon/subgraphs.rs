//! Subgraphs allow to tag tasks.

use super::next_task_id;
use super::now;
use crate::common_types::RawEvent;
/// We tag all the tasks that op makes as one subgraph.
///
/// `work_type` is a str tag and `work_amount` an integer specifying the expected algorithmic cost
/// (should not be zero).
/// As we know the work and execution time we can compute an execution speed for each subgraph.
/// When different graphs are tagged with the same tag we can then compare their speeds.
/// Slow graphs will see their displayed colors darkened.
/// You can also hover on tasks to display their speeds.
///
/// Example:
///
/// ```
/// use rayon_logs::{join, subgraph};
///
/// fn manual_max(slice: &[u32]) -> u32 {
///     if slice.len() < 200_000 {
///         subgraph("max", slice.len(), || slice.iter().max().cloned().unwrap())
///     } else {
///         let middle = slice.len() / 2;
///         let (left, right) = slice.split_at(middle);
///         let (mleft, mright) = join(|| manual_max(left), || manual_max(right));
///         std::cmp::max(mleft, mright)
///     }
/// }
///
/// let v: Vec<u32> = (0..2_000_000).collect();
/// let max = manual_max(&v);
/// assert_eq!(max, v.last().cloned().unwrap());
/// ```
///
/// <div>
/// <img
/// src="http://www-id.imag.fr/Laboratoire/Membres/Wagner_Frederic/images/downgraded_manual_max.svg"/>
/// </div>
///
/// Using it we obtain the graph below.
/// On the real file you can hover but javascript and toggle the display of the different tags but
/// it is disabled with rustdoc so I downgraded the file
/// for this display.
pub fn subgraph<OP, R>(work_type: &'static str, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    custom_subgraph(work_type, || (), |_| work_amount, op)
}

/// Tag a subgraph with a custom value.
/// The start function will be called just before running the graph and produce an S.
/// The end function will be called just after running the graph on this S and produce a usize
/// which will the be stored for display.
pub fn custom_subgraph<OP, R, START, END, S>(tag: &'static str, start: START, end: END, op: OP) -> R
where
    OP: FnOnce() -> R,
    START: FnOnce() -> S,
    END: FnOnce(S) -> usize,
{
    let s = start();
    start_subgraph(tag);
    let r = op();
    let measured_value = end(s);
    end_subgraph(tag, measured_value);
    r
}

/// Stop current task (virtually) and start a subgraph.
/// You most likely don't need to call this function directly but `subgraph` instead.
fn start_subgraph(tag: &'static str) {
    let subgraph_start_task_id = next_task_id();
    logs!(
        // log child's work and dependencies.
        RawEvent::Child(subgraph_start_task_id),
        // end current task
        RawEvent::TaskEnd(now()),
        // execute full sequential task
        RawEvent::TaskStart(subgraph_start_task_id, now()),
        RawEvent::SubgraphStart(tag)
    );
}

/// Stop current task (virtually) and end a subgraph.
/// You most likely don't need to call this function directly but `subgraph` instead.
fn end_subgraph(tag: &'static str, measured_value: usize) {
    let continuation_task_id = next_task_id();
    logs!(
        RawEvent::SubgraphEnd(tag, measured_value),
        RawEvent::Child(continuation_task_id),
        RawEvent::TaskEnd(now()),
        // start continuation task
        RawEvent::TaskStart(continuation_task_id, now())
    );
}
