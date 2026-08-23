//! Wakeup latency benchmark, shared by `sched-rr`/`sched-cfs` and
//! `sched-rt-fifo`.
//!
//! A high-priority task sleeps for a fixed duration while a lower-priority
//! CPU-bound task spins. We measure how much later than the requested deadline
//! the sleeper actually resumes:
//!
//! - Under `sched-rt-fifo`: the sleeper is woken by the timer softirq and, being
//!   higher priority than the spinning load, preempts it immediately -> small
//!   wakeup latency.
//! - Under `sched-rr`/`sched-cfs`: the sleeper is woken but has the same
//!   priority as the load, so it waits for a time slice -> larger wakeup
//!   latency.
//!
//! The same source runs under both scheduler configurations, giving a direct
//! before/after comparison of the realtime preemption behavior. QEMU timing is
//! not authoritative for real hardware, but the relative difference shows the
//! mechanism taking effect.

use std::{
    os::arceos::modules::ax_task::{self, TaskInner},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

static WAKE_LOAD_STOP: AtomicBool = AtomicBool::new(false);
static WAKE_DONE: AtomicBool = AtomicBool::new(false);
static WAKE_LATENCY_NS: AtomicU64 = AtomicU64::new(0);

const SLEEP_FOR: Duration = Duration::from_millis(5);

pub fn run_wakeup_latency_benchmark() -> crate::TestResult {
    WAKE_LOAD_STOP.store(false, Ordering::Release);
    WAKE_DONE.store(false, Ordering::Release);
    WAKE_LATENCY_NS.store(0, Ordering::Release);

    // CPU-bound lower-priority load: spins until told to stop.
    let load = {
        let task = TaskInner::new(
            move || {
                // Pure CPU-bound spin (no voluntary yield) so the sleeper's
                // wake must preempt this task to run.
                while !WAKE_LOAD_STOP.load(Ordering::Acquire) {
                    core::hint::spin_loop();
                }
            },
            "wake-lat-load".into(),
            ax_task::default_task_stack_size(),
        );
        ax_task::spawn_task_with(task, |task| task.set_sched_priority(0))
    };

    // Higher-priority sleeper: sleep, then record how late it resumed.
    let sleeper = {
        let task = TaskInner::new(
            move || {
                let start = Instant::now();
                thread::sleep(SLEEP_FOR);
                let elapsed = start.elapsed();
                // Latency beyond the requested sleep duration = wakeup latency.
                let latency = elapsed.checked_sub(SLEEP_FOR).unwrap_or_default();
                WAKE_LATENCY_NS.store(latency.as_nanos() as u64, Ordering::Release);
                WAKE_DONE.store(true, Ordering::Release);
            },
            "wake-lat-sleeper".into(),
            ax_task::default_task_stack_size(),
        );
        ax_task::spawn_task_with(task, |task| task.set_sched_priority(30))
    };

    let mut observed = false;
    for _ in 0..1_000_000 {
        if WAKE_DONE.load(Ordering::Acquire) {
            observed = true;
            break;
        }
        thread::yield_now();
    }
    WAKE_LOAD_STOP.store(true, Ordering::Release);
    load.join();
    sleeper.join();

    let latency = WAKE_LATENCY_NS.load(Ordering::Acquire);
    if !observed {
        return Err("wakeup latency benchmark: sleeper never resumed");
    }
    std::println!(
        "wakeup latency: {latency} ns (sleep {} ms)",
        SLEEP_FOR.as_millis()
    );
    Ok(())
}
