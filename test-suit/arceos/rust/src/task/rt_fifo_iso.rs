//! Realtime FIFO host-internal CPU affinity (`RT_CPUMASK`).
//!
//! With `RT_CPUMASK` configured at build time, [`ax_task::spawn_rt_task`]
//! pins realtime tasks to the reserved RT cores via [`rt_cpu_mask`]. This case
//! reserves CPU 1 for RT and verifies a realtime task spawned via
//! `spawn_rt_task` runs on CPU 1.
//!
//! Note: forcing *all* default tasks off the RT cores (full host-internal
//! isolation) requires task-class-aware default masks and is left as future
//! work; this case validates the RT-side affinity only.

use std::{
    os::arceos::modules::ax_task,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    thread,
};

static ISO_RT_CPU: AtomicU32 = AtomicU32::new(u32::MAX);
static ISO_RT_DONE: AtomicBool = AtomicBool::new(false);

pub fn run() -> crate::TestResult {
    if std::thread::available_parallelism().map_or(0, |c| c.get()) < 2 {
        return Err("sched-rt-fifo-iso requires at least 2 CPUs");
    }
    if ax_task::rt_cpu_mask().is_empty() {
        return Err("sched-rt-fifo-iso requires RT_CPUMASK to be configured at build time");
    }

    // Realtime task: pinned to the RT core(s) by spawn_rt_task.
    let rt = ax_task::spawn_rt_task(move || {
        ISO_RT_CPU.store(ax_task::current().cpu_id(), Ordering::Release);
        ISO_RT_DONE.store(true, Ordering::Release);
    });

    for _ in 0..100_000 {
        if ISO_RT_DONE.load(Ordering::Acquire) {
            break;
        }
        thread::yield_now();
    }

    let rt_cpu = ISO_RT_CPU.load(Ordering::Acquire);
    if rt_cpu != 1 {
        return Err("sched-rt-fifo-iso realtime task did not run on the reserved RT CPU 1");
    }

    rt.join();
    std::println!("sched-rt-fifo-iso RT affinity OK (rt=cpu{rt_cpu})");
    Ok(())
}
