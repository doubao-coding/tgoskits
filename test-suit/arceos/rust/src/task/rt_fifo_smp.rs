//! Realtime FIFO scheduling on SMP: cross-CPU wake via forced reschedule IPI.
//!
//! Under `sched-rt-fifo` + `smp`, making a higher-priority task runnable on
//! another CPU must use a forced (non-coalesced) reschedule IPI so the remote
//! CPU runs it without delay. This case pins a high-priority task to CPU 1 and
//! spawns it from the runner (CPU 0); the `add_task` path force-kicks CPU 1,
//! which schedules the high-priority task there.

use std::{
    os::arceos::modules::ax_task::{self, TaskInner},
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

static SMP_HIGH_DONE: AtomicBool = AtomicBool::new(false);

pub fn run() -> crate::TestResult {
    // The single-core `sched-rt-fifo` case covers the algorithm; this case
    // only makes sense with at least 2 CPUs.
    if std::thread::available_parallelism().map_or(0, |c| c.get()) < 2 {
        return Err("sched-rt-fifo-smp requires at least 2 CPUs");
    }

    // High-priority task pinned to CPU 1. Spawning it crosses CPU boundaries:
    // `add_task` force-kicks CPU 1 so it schedules this task there.
    let high = {
        let task = TaskInner::new(
            move || {
                SMP_HIGH_DONE.store(true, Ordering::Release);
            },
            "rt-fifo-smp-high".into(),
            ax_task::default_task_stack_size(),
        );
        ax_task::spawn_task_with(task, |task| {
            task.set_sched_priority(30);
            task.set_cpumask(ax_task::AxCpuMask::one_shot(1));
        })
    };

    let mut observed = false;
    for _ in 0..100_000 {
        if SMP_HIGH_DONE.load(Ordering::Acquire) {
            observed = true;
            break;
        }
        thread::yield_now();
    }
    if !observed {
        return Err(
            "sched-rt-fifo-smp cross-CPU spawn did not run the high-priority task on CPU 1",
        );
    }

    high.join();
    std::println!("sched-rt-fifo-smp cross-CPU wake OK");
    Ok(())
}
