//! Performance counters based subgraphs.

use rayon_core::custom_subgraph;
// we can now use performance counters to tag subgraphs
#[cfg(feature = "perf")]
use perfcnt::linux::PerfCounterBuilderLinux;
#[cfg(feature = "perf")]
use perfcnt::linux::{CacheId, CacheOpId, CacheOpResultId, HardwareEventType, SoftwareEventType};
#[cfg(feature = "perf")]
use perfcnt::{AbstractPerfCounter, PerfCounter};

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
/// You will have to import the events from rayon_logs
/// and to use the nightly version of the compiler.
/// note that It is **freaking slow**: 1 full second to set up the counter.
#[cfg(feature = "perf")]
pub fn subgraph_hardware_event<OP, R>(tag: &'static str, event: HardwareEventType, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    custom_subgraph(
        tag,
        || {
            let pc: PerfCounter = PerfCounterBuilderLinux::from_hardware_event(event)
                .exclude_idle()
                .exclude_kernel()
                .finish()
                .expect("Could not create counter");
            pc.start().expect("Can not start the counter");
            pc
        },
        |mut pc| {
            pc.stop().expect("Can not stop the counter");
            let counted_value = pc.read().unwrap() as usize;
            pc.reset().expect("Can not reset the counter");
            counted_value
        },
        op,
    )
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
/// You will have to import the events from rayon_logs
/// and to use the nightly version of the compiler
#[cfg(feature = "perf")]
pub fn subgraph_software_event<OP, R>(tag: &'static str, event: SoftwareEventType, op: OP) -> R
where
    OP: FnOnce() -> R,
{
    //TODO: avoid code duplication by abstracting over events
    custom_subgraph(
        tag,
        || {
            let pc: PerfCounter = PerfCounterBuilderLinux::from_software_event(event)
                .exclude_idle()
                .exclude_kernel()
                .finish()
                .expect("Could not create counter");
            pc.start().expect("Can not start the counter");
            pc
        },
        |mut pc| {
            pc.stop().expect("Can not stop the counter");
            let counted_value = pc.read().unwrap() as usize;
            pc.reset().expect("Can not reset the counter");
            counted_value
        },
        op,
    )
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
/// You will have to import the events from rayon_logs
/// and to use the nightly version of the compiler
///
#[cfg(feature = "perf")]
pub fn subgraph_cache_event<OP, R>(
    tag: &'static str,
    cache_id: CacheId,
    cache_op_id: CacheOpId,
    cache_op_result_id: CacheOpResultId,
    op: OP,
) -> R
where
    OP: FnOnce() -> R,
{
    //TODO: avoid code duplication by abstracting over events
    custom_subgraph(
        tag,
        || {
            let pc: PerfCounter = PerfCounterBuilderLinux::from_cache_event(
                cache_id,
                cache_op_id,
                cache_op_result_id,
            )
            .exclude_idle()
            .exclude_kernel()
            .finish()
            .expect("Could not create counter");
            pc.start().expect("Can not start the counter");
            pc
        },
        |mut pc| {
            pc.stop().expect("Can not stop the counter");
            let counted_value = pc.read().unwrap() as usize;
            pc.reset().expect("Can not reset the counter");
            counted_value
        },
        op,
    )
}
