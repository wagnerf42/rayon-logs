//! `LoggedPool` structure for logging raw tasks events.
#![macro_use]

#[cfg(feature = "perf")]
extern crate perfcnt;
#[cfg(feature = "perf")]
extern crate x86;
#[cfg(feature = "perf")]
use perfcnt::linux::PerfCounterBuilderLinux;
#[cfg(feature = "perf")]
use perfcnt::linux::{CacheId, CacheOpId, CacheOpResultId, HardwareEventType, SoftwareEventType};
#[cfg(feature = "perf")]
use perfcnt::{AbstractPerfCounter, PerfCounter};

use crate::log::RunLog;
use crate::raw_events::{RayonEvent, TaskId};
use crate::storage::Storage;
use crate::Comparator;
use crate::{scope, Scope};
use rayon;
use rayon::FnContext;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use time::precise_time_ns;

/// We use an atomic usize to generate unique ids for tasks.
pub(crate) static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);
/// We use an atomic usize to generate unique ids for iterators.
pub(crate) static NEXT_ITERATOR_ID: AtomicUsize = AtomicUsize::new(0);

/// get an id for a new task and increment global tasks counter.
pub fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// get an id for a new iterator and increment global iterators counter.
pub fn next_iterator_id() -> usize {
    NEXT_ITERATOR_ID.fetch_add(1, Ordering::SeqCst)
}

thread_local!(pub(crate) static LOGS: RefCell<Arc<Storage<RayonEvent>>> = RefCell::new(Arc::new(Storage::new())));

/// Add given event to logs of current thread.
pub(crate) fn log(event: RayonEvent) {
    LOGS.with(|l| l.borrow().push(event))
}

/// Logs several events at once (with decreased cost).
macro_rules! logs {
    ($($x:expr ), +) => {
        $crate::pool::LOGS.with(|l| {let thread_logs = l.borrow();
            $(
                thread_logs.push($x);
                )*
        })
    }
}

/// We tag all the tasks that op makes as one subgraph.
///
/// `work_type` is a str tag and `work_amount` an integer specifying the expected algorithmic cost
/// (should not be zero).
/// Subgraphs appear surrounded by grey boxes in svg files and you can hover other them to display
/// the tags.
/// If a subgraph is composed of only one task we use the work amount and running time to compute
/// a execution speed. When different tasks are tagged with the same tag we can compute each task's
/// speed. Slow tasks will see their displayed colors darken.
/// You can also hover on tasks to display their speeds.
///
/// Example:
///
/// ```
/// use rayon_logs::{subgraph, ThreadPoolBuilder};
/// fn invert(slice: &mut [u32]) {
///     subgraph("invert slice", slice.len(), || {
///         if slice.len() < 30_000 {
///             (0..slice.len() / 2).for_each(|i| slice.swap(i, slice.len() - i - 1))
///         } else {
///             let (left, right) = slice.split_at_mut(slice.len() / 2);
///             rayon_logs::join(|| invert(left), || invert(right));
///         }
///     })
/// }
///
/// let pool = ThreadPoolBuilder::new()
///     .num_threads(2)
///     .build()
///     .expect("failed creating pool");
/// pool.install(|| {
///     let mut v: Vec<u32> = subgraph("vector creation", 100_000, || (0..100_000).collect());
///     invert(&mut v);
///     assert_eq!(v[49_999], 25_000);
///     assert_eq!(v[50_000], 74_999);
/// });
/// ```
///
/// Using it we obtain the graph below. You can see each subgraph in a grey box and the green
/// inversion being slower (darker) than the red ones due to cache effects.
/// On the real file you can hover but javascript is disabled with rustdoc so I downgraded the file
/// for this display.
///
/// <svg viewBox="0 0 1920 1080" version="1.1" xmlns="http://www.w3.org/2000/svg">
/// <line x1="272.04318738039706" y1="476.1290322580646" x2="272.04318738039706" y2="487.74193548387103" stroke="black" stroke-width="2.0"/><line x1="272.04318738039706" y1="545.8064516129033" x2="272.04318738039706" y2="557.4193548387098" stroke="black" stroke-width="2.0"/><line x1="500.36505938627533" y1="476.1290322580646" x2="500.36505938627533" y2="487.74193548387103" stroke="black" stroke-width="2.0"/><line x1="500.36505938627533" y1="545.8064516129033" x2="500.36505938627533" y2="557.4193548387098" stroke="black" stroke-width="2.0"/><line x1="385.68677023747773" y1="406.4516129032259" x2="272.04318738039706" y2="418.06451612903237" stroke="black" stroke-width="2.0"/><line x1="385.68677023747773" y1="406.4516129032259" x2="500.36505938627533" y2="418.06451612903237" stroke="black" stroke-width="2.0"/><line x1="272.04318738039706" y1="615.483870967742" x2="385.68677023747773" y2="627.0967741935485" stroke="black" stroke-width="2.0"/><line x1="500.36505938627533" y1="615.483870967742" x2="385.68677023747773" y2="627.0967741935485" stroke="black" stroke-width="2.0"/><line x1="385.68677023747773" y1="336.7741935483872" x2="385.68677023747773" y2="348.38709677419365" stroke="black" stroke-width="2.0"/><line x1="385.68677023747773" y1="685.1612903225807" x2="385.68677023747773" y2="696.7741935483872" stroke="black" stroke-width="2.0"/><line x1="751.9379756589076" y1="476.1290322580646" x2="751.9379756589076" y2="487.74193548387103" stroke="black" stroke-width="2.0"/><line x1="751.9379756589076" y1="545.8064516129033" x2="751.9379756589076" y2="557.4193548387098" stroke="black" stroke-width="2.0"/><line x1="1003.7546448560308" y1="476.1290322580646" x2="1003.7546448560308" y2="487.74193548387103" stroke="black" stroke-width="2.0"/><line x1="1003.7546448560308" y1="545.8064516129033" x2="1003.7546448560308" y2="557.4193548387098" stroke="black" stroke-width="2.0"/><line x1="865.8253114404793" y1="406.4516129032259" x2="751.9379756589076" y2="418.06451612903237" stroke="black" stroke-width="2.0"/><line x1="865.8253114404793" y1="406.4516129032259" x2="1003.7546448560308" y2="418.06451612903237" stroke="black" stroke-width="2.0"/><line x1="751.9379756589076" y1="615.483870967742" x2="865.8253114404793" y2="627.0967741935485" stroke="black" stroke-width="2.0"/><line x1="1003.7546448560308" y1="615.483870967742" x2="865.8253114404793" y2="627.0967741935485" stroke="black" stroke-width="2.0"/><line x1="865.8253114404793" y1="336.7741935483872" x2="865.8253114404793" y2="348.38709677419365" stroke="black" stroke-width="2.0"/><line x1="865.8253114404793" y1="685.1612903225807" x2="865.8253114404793" y2="696.7741935483872" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="267.09677419354847" x2="385.68677023747773" y2="278.70967741935493" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="267.09677419354847" x2="865.8253114404793" y2="278.70967741935493" stroke="black" stroke-width="2.0"/><line x1="385.68677023747773" y1="754.8387096774194" x2="637.503439434601" y2="766.4516129032259" stroke="black" stroke-width="2.0"/><line x1="865.8253114404793" y1="754.8387096774194" x2="637.503439434601" y2="766.4516129032259" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="58.06451612903226" x2="637.503439434601" y2="69.67741935483872" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="127.74193548387099" x2="637.503439434601" y2="139.35483870967747" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="197.41935483870972" x2="637.503439434601" y2="209.03225806451618" stroke="black" stroke-width="2.0"/><line x1="637.503439434601" y1="824.516129032258" x2="637.503439434601" y2="836.1290322580645" stroke="black" stroke-width="2.0"/><rect x="620.2019563452193" y="0" width="34.602966178763545" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="620.2019563452193" y="0" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="34.602966178763545" begin="0s" dur="0.9913586287820845s" fill="freeze"/>
/// </rect>
/// <rect x="0" y="69.67741935483872" width="1275.006878869202" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="0" y="69.67741935483872" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1275.006878869202" begin="1.033259065363731s" dur="36.52834455270571s" fill="freeze"/>
/// </rect>
/// <rect x="636.200107470996" y="139.35483870967747" width="2.606663927209905" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="636.200107470996" y="139.35483870967747" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.606663927209905" begin="37.58526168770398s" dur="0.07467968968973725s" fill="freeze"/>
/// </rect>
/// <rect x="157.36489823159948" y="209.03225806451618" width="960.277082406003" height="615.4838709677418" fill="rgba(51,51,51,0.3)"/>
/// <rect x="635.7921944136846" y="209.03225806451618" width="3.422490041832852" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="635.7921944136846" y="209.03225806451618" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="3.422490041832852" begin="37.66849248690018s" dur="0.09805272234072371s" fill="freeze"/>
/// </rect>
/// <rect x="384.1993799431347" y="278.70967741935493" width="2.9747805886861127" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="384.1993799431347" y="278.70967741935493" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.9747805886861127" begin="38.38479042655785s" dur="0.08522605808103602s" fill="freeze"/>
/// </rect>
/// <rect x="157.36489823159948" y="348.38709677419365" width="456.64374401175655" height="336.7741935483872" fill="rgba(51,51,51,0.3)"/>
/// <rect x="384.94556236504593" y="348.38709677419365" width="1.4824157448636481" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="384.94556236504593" y="348.38709677419365" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.4824157448636481" begin="38.47771248319469s" dur="0.0424705105487437s" fill="freeze"/>
/// </rect>
/// <rect x="270.9686846928449" y="418.06451612903237" width="2.149005375104349" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="270.9686846928449" y="418.06451612903237" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.149005375104349" begin="38.54526624829572s" dur="0.06156798844650093s" fill="freeze"/>
/// </rect>
/// <rect x="157.36489823159948" y="487.74193548387103" width="229.35657829759518" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="157.36489823159948" y="487.74193548387103" width="0" height="58.06451612903226" fill="rgba(255,0,0,0.9939849)">
/// <animate attributeType="XML" attributeName="width" from="0" to="229.35657829759518" begin="38.61424519831448s" dur="6.570957581746232s" fill="freeze"/>
/// </rect>
/// <rect x="269.3967270573519" y="557.4193548387098" width="5.292920646090341" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="269.3967270573519" y="557.4193548387098" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="5.292920646090341" begin="45.19318381560007s" dur="0.1516396752478634s" fill="freeze"/>
/// </rect>
/// <rect x="499.1363456648615" y="418.06451612903237" width="2.4574274428276586" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="499.1363456648615" y="418.06451612903237" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.4574274428276586" begin="45.497603314029995s" dur="0.07040413493650802s" fill="freeze"/>
/// </rect>
/// <rect x="386.72147652919466" y="487.74193548387103" width="227.28716571416135" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="386.72147652919466" y="487.74193548387103" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="227.28716571416135" begin="45.575703447522315s" dur="6.51166988916812s" fill="freeze"/>
/// </rect>
/// <rect x="499.76313889926695" y="557.4193548387098" width="1.2038409740167881" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="499.76313889926695" y="557.4193548387098" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.2038409740167881" begin="52.0953543722298s" dur="0.034489475009382466s" fill="freeze"/>
/// </rect>
/// <rect x="384.65206394576086" y="627.0967741935485" width="2.0694125834338175" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="384.65206394576086" y="627.0967741935485" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.0694125834338175" begin="52.15036651005468s" dur="0.05928769257811201s" fill="freeze"/>
/// </rect>
/// <rect x="385.13956979474284" y="696.7741935483872" width="1.0944008854698073" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="385.13956979474284" y="696.7741935483872" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.0944008854698073" begin="52.2173502011886s" dur="0.031354068190347695s" fill="freeze"/>
/// </rect>
/// <rect x="858.582367398461" y="278.70967741935493" width="14.485888084036723" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="858.582367398461" y="278.70967741935493" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="14.485888084036723" begin="46.156608819994396s" dur="0.4150138480467841s" fill="freeze"/>
/// </rect>
/// <rect x="614.008642243356" y="348.38709677419365" width="503.63333839424655" height="336.7741935483872" fill="rgba(51,51,51,0.3)"/>
/// <rect x="864.3329465966568" y="348.38709677419365" width="2.984729687644929" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="864.3329465966568" y="348.38709677419365" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.984729687644929" begin="46.596990959577006s" dur="0.08551109506458462s" fill="freeze"/>
/// </rect>
/// <rect x="750.783880179685" y="418.06451612903237" width="2.308190958445412" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="750.783880179685" y="418.06451612903237" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.308190958445412" begin="46.75433137449584s" dur="0.06612858018327879s" fill="freeze"/>
/// </rect>
/// <rect x="614.008642243356" y="487.74193548387103" width="275.85866683110316" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="614.008642243356" y="487.74193548387103" width="0" height="58.06451612903226" fill="rgba(0,255,0,0.8826174)">
/// <animate attributeType="XML" attributeName="width" from="0" to="275.85866683110316" begin="46.827870916251385s" dur="7.90322044285246s" fill="freeze"/>
/// </rect>
/// <rect x="750.6843891900968" y="557.4193548387098" width="2.5071729376217404" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="750.6843891900968" y="557.4193548387098" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.5071729376217404" begin="54.75104394795225s" dur="0.07182931985425109s" fill="freeze"/>
/// </rect>
/// <rect x="1002.5458293325347" y="418.06451612903237" width="2.4176310469923927" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1002.5458293325347" y="418.06451612903237" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.4176310469923927" begin="52.50466748060561s" dur="0.06926398700231355s" fill="freeze"/>
/// </rect>
/// <rect x="889.8673090744592" y="487.74193548387103" width="227.77467156314336" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="889.8673090744592" y="487.74193548387103" width="0" height="58.06451612903226" fill="rgba(255,0,0,0.9985731)">
/// <animate attributeType="XML" attributeName="width" from="0" to="227.77467156314336" begin="52.58105739219664s" dur="6.525636701362002s" fill="freeze"/>
/// </rect>
/// <rect x="1003.073131577352" y="557.4193548387098" width="1.363026557357851" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1003.073131577352" y="557.4193548387098" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.363026557357851" begin="59.114675129098s" dur="0.039050066746160315s" fill="freeze"/>
/// </rect>
/// <rect x="864.7259360055301" y="627.0967741935485" width="2.198750869898431" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="864.7259360055301" y="627.0967741935485" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.198750869898431" begin="59.34726530767367s" dur="0.06299317336424401s" fill="freeze"/>
/// </rect>
/// <rect x="865.1089763154445" y="696.7741935483872" width="1.432670250069566" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="865.1089763154445" y="696.7741935483872" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.432670250069566" begin="59.41823951657727s" dur="0.04104532563100062s" fill="freeze"/>
/// </rect>
/// <rect x="636.3990894501724" y="766.4516129032259" width="2.2086999688572475" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="636.3990894501724" y="766.4516129032259" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.2086999688572475" begin="59.67477280177103s" dur="0.06327821034779263s" fill="freeze"/>
/// </rect>
/// <rect x="633.0711158484484" y="836.1290322580645" width="8.864647172305439" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="633.0711158484484" y="836.1290322580645" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="8.864647172305439" begin="59.74603204765818s" dur="0.25396795234181635s" fill="freeze"/>
/// </rect>
/// <rect x="127.50068788692022" y="952.258064516129" width="1.4625175469460152" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="127.50068788692022" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1.4625175469460152" begin="0.9913586287820845s" dur="0.041900436581646466s" fill="freeze"/>
/// </rect>
/// <rect x="128.96320543386622" y="952.258064516129" width="0.8257752135817638" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="128.96320543386622" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.8257752135817638" begin="37.56160361806945s" dur="0.02365806963453508s" fill="freeze"/>
/// </rect>
/// <rect x="129.788980647448" y="952.258064516129" width="0.2984729687644929" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="129.788980647448" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.2984729687644929" begin="37.659941377393714s" dur="0.008551109506458463s" fill="freeze"/>
/// </rect>
/// <rect x="130.08745361621249" y="952.258064516129" width="21.57959564167284" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="130.08745361621249" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="21.57959564167284" begin="37.766545209240896s" dur="0.6182452173169469s" fill="freeze"/>
/// </rect>
/// <rect x="151.66704925788534" y="952.258064516129" width="0.26862567188804365" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="151.66704925788534" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.26862567188804365" begin="38.47001648463888s" dur="0.007695998555812616s" fill="freeze"/>
/// </rect>
/// <rect x="151.93567492977337" y="952.258064516129" width="0.875520708375846" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="151.93567492977337" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.875520708375846" begin="38.52018299374344s" dur="0.025083254552278157s" fill="freeze"/>
/// </rect>
/// <rect x="152.81119563814923" y="952.258064516129" width="0.2586765729292272" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="152.81119563814923" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.2586765729292272" begin="38.606834236742216s" dur="0.007410961572264002s" fill="freeze"/>
/// </rect>
/// <rect x="153.06987221107843" y="952.258064516129" width="0.27857477084686005" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="153.06987221107843" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.27857477084686005" begin="45.18520278006071s" dur="0.007981035539361232s" fill="freeze"/>
/// </rect>
/// <rect x="153.3484469819253" y="952.258064516129" width="5.332717041925607" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="153.3484469819253" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="5.332717041925607" begin="45.344823490847936s" dur="0.15277982318205788s" fill="freeze"/>
/// </rect>
/// <rect x="158.6811640238509" y="952.258064516129" width="0.26862567188804365" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="158.6811640238509" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.26862567188804365" begin="45.568007448966505s" dur="0.007695998555812616s" fill="freeze"/>
/// </rect>
/// <rect x="127.50068788692022" y="1021.9354838709677" width="1611.0774925990622" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="127.50068788692022" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="1611.0774925990622" begin="0s" dur="46.156608819994396s" fill="freeze"/>
/// </rect>
/// <rect x="1738.5781804859823" y="1021.9354838709677" width="0.8854698073346623" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1738.5781804859823" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.8854698073346623" begin="46.57162266804118s" dur="0.025368291535826773s" fill="freeze"/>
/// </rect>
/// <rect x="1739.463650293317" y="1021.9354838709677" width="2.5071729376217404" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1739.463650293317" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="2.5071729376217404" begin="46.68250205464159s" dur="0.07182931985425109s" fill="freeze"/>
/// </rect>
/// <rect x="1741.9708232309388" y="1021.9354838709677" width="0.2586765729292272" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1741.9708232309388" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.2586765729292272" begin="46.82045995467912s" dur="0.007410961572264002s" fill="freeze"/>
/// </rect>
/// <rect x="158.94978969573896" y="952.258064516129" width="0.27857477084686005" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="158.94978969573896" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.27857477084686005" begin="52.08737333669043s" dur="0.007981035539361232s" fill="freeze"/>
/// </rect>
/// <rect x="159.22836446658582" y="952.258064516129" width="0.716335125034783" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="159.22836446658582" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.716335125034783" begin="52.12984384723918s" dur="0.02052266281550031s" fill="freeze"/>
/// </rect>
/// <rect x="159.9446995916206" y="952.258064516129" width="0.26862567188804365" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="159.9446995916206" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.26862567188804365" begin="52.20965420263279s" dur="0.007695998555812616s" fill="freeze"/>
/// </rect>
/// <rect x="160.21332526350864" y="952.258064516129" width="8.934290865017156" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="160.21332526350864" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="8.934290865017156" begin="52.248704269378955s" dur="0.25596321122665666s" fill="freeze"/>
/// </rect>
/// <rect x="169.1476161285258" y="952.258064516129" width="0.24872747397041076" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="169.1476161285258" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.24872747397041076" begin="52.57393146760792s" dur="0.007125924588715386s" fill="freeze"/>
/// </rect>
/// <rect x="1742.229499803868" y="1021.9354838709677" width="0.6964369271171501" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1742.229499803868" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.6964369271171501" begin="54.73109135910384s" dur="0.01995258884840308s" fill="freeze"/>
/// </rect>
/// <rect x="169.39634360249622" y="952.258064516129" width="0.27857477084686005" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="169.39634360249622" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.27857477084686005" begin="59.10669409355864s" dur="0.007981035539361232s" fill="freeze"/>
/// </rect>
/// <rect x="1742.925936730985" y="1021.9354838709677" width="157.9220477732932" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1742.925936730985" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="157.9220477732932" begin="54.8228732678065s" dur="4.524392039867172s" fill="freeze"/>
/// </rect>
/// <rect x="1900.8479845042784" y="1021.9354838709677" width="0.27857477084686005" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1900.8479845042784" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.27857477084686005" begin="59.410258481037914s" dur="0.007981035539361232s" fill="freeze"/>
/// </rect>
/// <rect x="169.67491837334308" y="952.258064516129" width="18.186952896716434" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="169.67491837334308" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="18.186952896716434" begin="59.15372519584416s" dur="0.521047605926869s" fill="freeze"/>
/// </rect>
/// <rect x="187.8618712700595" y="952.258064516129" width="0.27857477084686005" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="187.8618712700595" y="952.258064516129" width="0" height="58.06451612903226" fill="rgba(255,0,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="0.27857477084686005" begin="59.738051012118824s" dur="0.007981035539361232s" fill="freeze"/>
/// </rect>
/// <rect x="1901.1265592751251" y="1021.9354838709677" width="18.87344072487477" height="58.06451612903226" fill="black"/>
/// <rect class="task4629870818619103512" x="1901.1265592751251" y="1021.9354838709677" width="0" height="58.06451612903226" fill="rgba(0,255,0,1)">
/// <animate attributeType="XML" attributeName="width" from="0" to="18.87344072487477" begin="59.45928484220828s" dur="0.5407151577917235s" fill="freeze"/>
/// </rect>
/// </svg>
pub fn subgraph<OP, R>(work_type: &'static str, work_amount: usize, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    let subgraph_start_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    logs!(
        // log child's work and dependencies.
        RayonEvent::Child(subgraph_start_task_id),
        // end current task
        RayonEvent::TaskEnd(precise_time_ns()),
        // execute full sequential task
        RayonEvent::TaskStart(subgraph_start_task_id, precise_time_ns()),
        RayonEvent::SubgraphStart(work_type, work_amount)
    );
    let r = op();
    logs!(
        RayonEvent::SubgraphEnd(work_type),
        RayonEvent::Child(continuation_task_id),
        RayonEvent::TaskEnd(precise_time_ns()),
        // start continuation task
        RayonEvent::TaskStart(continuation_task_id, precise_time_ns(),)
    );
    r
}

/// Same as the subgraph function, but we can log a hardware event
///
/// (from: https://github.com/gz/rust-perfcnt)
///
/// Events:
///
/// * ```HardwareEventType::CPUCycles```
///
/// * ```HardwareEventType::Instructions```
///
/// * ```HardwareEventType::CacheReferences```
///
/// * ```HardwareEventType::CacheMisses```
///
/// * ```HardwareEventType::BranchInstructions```
///
/// * ```HardwareEventType::BranchMisses```
///
/// * ```HardwareEventType::BusCycles```
///
/// * ```HardwareEventType::StalledCyclesFrontend```
///
/// * ```HardwareEventType::StalledCyclesBackend```
///
/// * ```HardwareEventType::RefCPUCycles```
///
/// You will have to import the rayon_logs prelude to use these events
/// and to use the nightly version of the compiler
///
/// You can give a label to the event you are logging
#[cfg(feature = "perf")]
pub fn subgraph_perf_hw<OP, R>(
    work_type: &'static str,
    hardware_event: HardwareEventType,
    perf_name: &'static str,
    op: OP,
) -> R
where
    OP: FnOnce() -> R,
{
    let subgraph_start_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    let mut pc: PerfCounter = PerfCounterBuilderLinux::from_hardware_event(hardware_event)
        .exclude_idle()
        .exclude_kernel()
        .finish()
        .expect("Could not create counter");

    logs!(
        // log child's work and dependencies.
        RayonEvent::Child(subgraph_start_task_id),
        // end current task
        RayonEvent::TaskEnd(precise_time_ns()),
        // execute full sequential task
        RayonEvent::TaskStart(subgraph_start_task_id, precise_time_ns()),
        RayonEvent::SubgraphPerfStart(work_type)
    );

    pc.start().expect("Can not start the counter");
    let r = op();
    pc.stop().expect("Can not stop the counter");;
    let cache_misses: usize = pc.read().unwrap() as usize;

    logs!(
        RayonEvent::SubgraphPerfEnd(work_type, cache_misses, perf_name),
        RayonEvent::Child(continuation_task_id),
        RayonEvent::TaskEnd(precise_time_ns()),
        // start continuation task
        RayonEvent::TaskStart(continuation_task_id, precise_time_ns(),)
    );
    pc.reset().expect("Can not reset the counter");
    r
}

/// Same as the subgraph function, but we can log a software event
///
/// (from: https://github.com/gz/rust-perfcnt)
///
/// Events:
///
/// * ```SoftwareEventType::CpuClock```
///
/// * ```SoftwareEventType::TaskClock```
///
/// * ```SoftwareEventType::PageFaults```
///
/// * ```SoftwareEventType::CacheMisses```
///
/// * ```SoftwareEventType::ContextSwitches```
///
/// * ```SoftwareEventType::CpuMigrations```
///
/// * ```SoftwareEventType::PageFaultsMin```
///
/// * ```SoftwareEventType::PageFaultsMin```
///
/// * ```SoftwareEventType::PageFaultsMaj```
///
/// * ```SoftwareEventType::AlignmentFaults```
///
/// * ```SoftwareEventType::EmulationFaults```
///
/// You will have to import the rayon_logs prelude to use these events
/// and to use the nightly version of the compiler
///
/// You can give a label to the event you are logging

#[cfg(feature = "perf")]
pub fn subgraph_perf_sw<OP, R>(
    work_type: &'static str,
    software_event: SoftwareEventType,
    perf_name: &'static str,
    op: OP,
) -> R
where
    OP: FnOnce() -> R,
{
    let subgraph_start_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    let mut pc: PerfCounter = PerfCounterBuilderLinux::from_software_event(software_event)
        .exclude_idle()
        .exclude_kernel()
        .finish()
        .expect("Could not create counter");

    logs!(
        // log child's work and dependencies.
        RayonEvent::Child(subgraph_start_task_id),
        // end current task
        RayonEvent::TaskEnd(precise_time_ns()),
        // execute full sequential task
        RayonEvent::TaskStart(subgraph_start_task_id, precise_time_ns()),
        RayonEvent::SubgraphPerfStart(work_type)
    );

    pc.start().expect("Can not start the counter");
    let r = op();
    pc.stop().expect("Can not stop the counter");;
    let cache_misses: usize = pc.read().unwrap() as usize;

    logs!(
        RayonEvent::SubgraphPerfEnd(work_type, cache_misses, perf_name),
        RayonEvent::Child(continuation_task_id),
        RayonEvent::TaskEnd(precise_time_ns()),
        // start continuation task
        RayonEvent::TaskStart(continuation_task_id, precise_time_ns(),)
    );
    pc.reset().expect("Can not reset the counter");
    r
}

/// Same as the subgraph function, but we can log a cache event
///
/// (from: https://github.com/gz/rust-perfcnt)
///
/// CacheId:
///
/// * ```CacheId::L1D```
///
/// * ```CacheId::L1I```
///
/// * ```CacheId::LL```
///
/// * ```CacheId::DTLB```
///
/// * ```CacheId::ITLB```
///
/// * ```CacheId::BPU```
///
/// * ```CacheId::Node```
///
/// CacheOpId:
///
/// * ```CacheOpId::Read```
///
/// * ```CacheOpId::Write```
///
/// * ```CacheOpId::Prefetch```
///
/// CacheOpResultId:
///
/// * ```CacheOpResultId::Access```
///
/// * ```CacheOpResultId::Miss```
///
///
/// You will have to import the rayon_logs prelude to use these events
/// and to use the nightly version of the compiler
///
/// You can give a label to the event you are logging

#[cfg(feature = "perf")]
pub fn subgraph_perf_cache<OP, R>(
    work_type: &'static str,
    cache_id: CacheId,
    cache_op_id: CacheOpId,
    cache_op_result_id: CacheOpResultId,
    perf_name: &'static str,
    op: OP,
) -> R
where
    OP: FnOnce() -> R,
{
    let subgraph_start_task_id = next_task_id();
    let continuation_task_id = next_task_id();
    let mut pc: PerfCounter =
        PerfCounterBuilderLinux::from_cache_event(cache_id, cache_op_id, cache_op_result_id)
            .exclude_idle()
            .exclude_kernel()
            .finish()
            .expect("Could not create counter");

    logs!(
        // log child's work and dependencies.
        RayonEvent::Child(subgraph_start_task_id),
        // end current task
        RayonEvent::TaskEnd(precise_time_ns()),
        // execute full sequential task
        RayonEvent::TaskStart(subgraph_start_task_id, precise_time_ns()),
        RayonEvent::SubgraphPerfStart(work_type)
    );

    pc.start().expect("Can not start the counter");
    let r = op();
    pc.stop().expect("Can not stop the counter");;
    let cache_misses: usize = pc.read().unwrap() as usize;

    logs!(
        RayonEvent::SubgraphPerfEnd(work_type, cache_misses, perf_name),
        RayonEvent::Child(continuation_task_id),
        RayonEvent::TaskEnd(precise_time_ns()),
        // start continuation task
        RayonEvent::TaskStart(continuation_task_id, precise_time_ns(),)
    );
    pc.reset().expect("Can not reset the counter");
    r
}
/// Identical to `join`, except that the closures have a parameter
/// that provides context for the way the closure has been called,
/// especially indicating whether they're executing on a different
/// thread than where `join_context` was called.  This will occur if
/// the second job is stolen by a different thread, or if
/// `join_context` was called from outside the thread pool to begin
/// with.
pub fn join_context<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce(FnContext) -> RA + Send,
    B: FnOnce(FnContext) -> RB + Send,
    RA: Send,
    RB: Send,
{
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = |c| {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a(c);
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    let id_b = next_task_id();
    let cb = |c| {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b(c);
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    logs!(
        RayonEvent::Child(id_a),
        RayonEvent::Child(id_b),
        RayonEvent::TaskEnd(precise_time_ns())
    );
    let r = rayon::join_context(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

/// Takes two closures and *potentially* runs them in parallel. It
/// returns a pair of the results from those closures.
///
/// Conceptually, calling `join()` is similar to spawning two threads,
/// one executing each of the two closures. However, the
/// implementation is quite different and incurs very low
/// overhead. The underlying technique is called "work stealing": the
/// Rayon runtime uses a fixed pool of worker threads and attempts to
/// only execute code in parallel when there are idle CPUs to handle
/// it.
///
/// When `join` is called from outside the thread pool, the calling
/// thread will block while the closures execute in the pool.  When
/// `join` is called within the pool, the calling thread still actively
/// participates in the thread pool. It will begin by executing closure
/// A (on the current thread). While it is doing that, it will advertise
/// closure B as being available for other threads to execute. Once closure A
/// has completed, the current thread will try to execute closure B;
/// if however closure B has been stolen, then it will look for other work
/// while waiting for the thief to fully execute closure B. (This is the
/// typical work-stealing strategy).
///
/// # Examples
///
/// This example uses join to perform a quick-sort (note this is not a
/// particularly optimized implementation: if you **actually** want to
/// sort for real, you should prefer [the `par_sort` method] offered
/// by Rayon).
///
/// [the `par_sort` method]: ../rayon/slice/trait.ParallelSliceMut.html#method.par_sort
///
/// ```rust
/// let mut v = vec![5, 1, 8, 22, 0, 44];
/// quick_sort(&mut v);
/// assert_eq!(v, vec![0, 1, 5, 8, 22, 44]);
///
/// fn quick_sort<T:PartialOrd+Send>(v: &mut [T]) {
///    if v.len() > 1 {
///        let mid = partition(v);
///        let (lo, hi) = v.split_at_mut(mid);
///        rayon::join(|| quick_sort(lo),
///                    || quick_sort(hi));
///    }
/// }
///
/// // Partition rearranges all items `<=` to the pivot
/// // item (arbitrary selected to be the last item in the slice)
/// // to the first half of the slice. It then returns the
/// // "dividing point" where the pivot is placed.
/// fn partition<T:PartialOrd+Send>(v: &mut [T]) -> usize {
///     let pivot = v.len() - 1;
///     let mut i = 0;
///     for j in 0..pivot {
///         if v[j] <= v[pivot] {
///             v.swap(i, j);
///             i += 1;
///         }
///     }
///     v.swap(i, pivot);
///     i
/// }
/// ```
///
/// # Warning about blocking I/O
///
/// The assumption is that the closures given to `join()` are
/// CPU-bound tasks that do not perform I/O or other blocking
/// operations. If you do perform I/O, and that I/O should block
/// (e.g., waiting for a network request), the overall performance may
/// be poor.  Moreover, if you cause one closure to be blocked waiting
/// on another (for example, using a channel), that could lead to a
/// deadlock.
///
/// # Panics
///
/// No matter what happens, both closures will always be executed.  If
/// a single closure panics, whether it be the first or second
/// closure, that panic will be propagated and hence `join()` will
/// panic with the same panic value. If both closures panic, `join()`
/// will panic with the panic value from the first closure.
pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let id_c = next_task_id();
    let id_a = next_task_id();
    let ca = || {
        log(RayonEvent::TaskStart(id_a, precise_time_ns()));
        let result = oper_a();
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    let id_b = next_task_id();
    let cb = || {
        log(RayonEvent::TaskStart(id_b, precise_time_ns()));
        let result = oper_b();
        logs!(
            RayonEvent::Child(id_c),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        result
    };

    logs!(
        RayonEvent::Child(id_a),
        RayonEvent::Child(id_b),
        RayonEvent::TaskEnd(precise_time_ns())
    );
    let r = rayon::join(ca, cb);
    log(RayonEvent::TaskStart(id_c, precise_time_ns()));
    r
}

// small global counter to increment file names
static INSTALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// We wrap rayon's pool into our own struct to overload the install method.
pub struct ThreadPool {
    pub(crate) logs: Arc<Mutex<Vec<Arc<Storage<RayonEvent>>>>>,
    pub(crate) pool: rayon::ThreadPool,
}

impl ThreadPool {
    /// Reset all logs and counters to initial condition.
    fn reset(&self) {
        NEXT_TASK_ID.store(0, Ordering::SeqCst);
        NEXT_ITERATOR_ID.store(0, Ordering::SeqCst);
        let logs = &*self.logs.lock().unwrap(); // oh yeah baby
        for log in logs {
            log.clear();
        }
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    /// After running, we post-process the logs and return a `RunLog` together with the closure's
    /// result.
    pub fn logging_install<OP, R>(&self, op: OP) -> (R, RunLog)
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        self.reset();
        let id = next_task_id();
        let c = || {
            log(RayonEvent::TaskStart(id, precise_time_ns()));
            let result = op();
            log(RayonEvent::TaskEnd(precise_time_ns()));
            result
        };
        let start = precise_time_ns();
        let r = self.pool.install(c);
        let log = RunLog::new(
            NEXT_TASK_ID.load(Ordering::Relaxed),
            NEXT_ITERATOR_ID.load(Ordering::Relaxed),
            &*self.logs.lock().unwrap(),
            start,
        );
        (r, log)
    }

    /// Creates a scope that executes within this thread-pool.
    /// Equivalent to `self.install(|| scope(...))`.
    ///
    /// See also: [the `scope()` function][scope].
    ///
    /// [scope]: fn.scope.html
    pub fn scope<'scope, OP, R>(&self, op: OP) -> R
    where
        OP: for<'s> FnOnce(&'s Scope<'scope>) -> R + 'scope + Send,
        R: Send,
    {
        self.install(|| scope(op))
    }

    /// Execute given closure in the thread pool, logging it's task as the initial one.
    /// After running, we save a json file with filename being an incremental counter.
    pub fn install<OP, R>(&self, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        let (r, log) = self.logging_install(op);
        log.save(format!(
            "log_{}.json",
            INSTALL_COUNT.fetch_add(1, Ordering::SeqCst)
        ))
        .expect("saving json failed");
        r
    }

    ///This function simply returns a comparator that allows us to add algorithms for comparison.
    pub fn compare(&self) -> Comparator {
        Comparator::new(self)
    }
}
