//! Realtime verification benchmark, shared by `sched-rr` and `sched-rt-fifo`.
//!
//! Measures the determinism of periodic-task wakeup under a CPU-bound load,
//! covering the task-1 verification metrics that can be exercised on a single
//! scheduler without external hardware:
//!   - scheduling/wakeup latency: how late a sleeper resumes past its requested
//!     sleep deadline, sampled over N periods;
//!   - jitter: max - min latency across samples;
//!   - max latency: worst-case wakeup latency (the worst-case response figure
//!     task-1 asks for);
//!   - long-time stability / deadline misses: how many periods exceed a
//!     deadline threshold over the run.
//!
//! The same source runs under both scheduler configurations for a direct
//! before/after comparison: `sched-rt-fifo` preempts the spinning load, so its
//! latency/jitter/misses are low; `sched-rr` time-slices against the load, so
//! they are high. QEMU timing is not authoritative for real hardware but the
//! relative difference shows the mechanism taking effect.

use std::{
    os::arceos::modules::ax_task::{self, TaskInner},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

static VERIFY_STOP: AtomicBool = AtomicBool::new(false);
static VERIFY_DONE: AtomicBool = AtomicBool::new(false);

// Aggregated statistics published by the sleeper, then printed by the runner.
// Packed as raw u64 because atomics only carry 64 bits; the sleeper writes them
// once at the end under the done flag.
static VERIFY_MIN_NS: AtomicU64 = AtomicU64::new(u64::MAX);
static VERIFY_MAX_NS: AtomicU64 = AtomicU64::new(0);
static VERIFY_SUM_NS: AtomicU64 = AtomicU64::new(0);
static VERIFY_COUNT: AtomicU64 = AtomicU64::new(0);
static VERIFY_MISSES: AtomicU64 = AtomicU64::new(0);

const PERIOD: Duration = Duration::from_millis(2);
const SAMPLES: u64 = 200;
// A deadline miss is a wakeup later than 5x the requested period; under
// `sched-rt-fifo` this never triggers, under `sched-rr` it fires often.
const MISS_THRESHOLD: Duration = Duration::from_millis(10);

pub fn run_realtime_verification() -> crate::TestResult {
    VERIFY_STOP.store(false, Ordering::Release);
    VERIFY_DONE.store(false, Ordering::Release);
    VERIFY_MIN_NS.store(u64::MAX, Ordering::Release);
    VERIFY_MAX_NS.store(0, Ordering::Release);
    VERIFY_SUM_NS.store(0, Ordering::Release);
    VERIFY_COUNT.store(0, Ordering::Release);
    VERIFY_MISSES.store(0, Ordering::Release);

    // CPU-bound load competing for the CPU while the sleeper waits.
    let load = {
        let task = TaskInner::new(
            move || {
                while !VERIFY_STOP.load(Ordering::Acquire) {
                    core::hint::spin_loop();
                }
            },
            "rt-verify-load".into(),
            ax_task::default_task_stack_size(),
        );
        ax_task::spawn_task_with(task, |task| task.set_sched_priority(0))
    };

    // Periodic sleeper: the high-priority realtime task under test.
    let sleeper = {
        let task = TaskInner::new(
            move || {
                let mut min = u64::MAX;
                let mut max = 0u64;
                let mut sum = 0u64;
                let mut misses = 0u64;
                for _ in 0..SAMPLES {
                    let start = Instant::now();
                    thread::sleep(PERIOD);
                    let elapsed = start.elapsed();
                    let latency = elapsed.checked_sub(PERIOD).unwrap_or_default();
                    let ns = u64::try_from(latency.as_nanos()).unwrap_or(u64::MAX);
                    if ns < min {
                        min = ns;
                    }
                    if ns > max {
                        max = ns;
                    }
                    sum += ns;
                    if latency > MISS_THRESHOLD {
                        misses += 1;
                    }
                }
                VERIFY_MIN_NS.store(min, Ordering::Release);
                VERIFY_MAX_NS.store(max, Ordering::Release);
                VERIFY_SUM_NS.store(sum, Ordering::Release);
                VERIFY_COUNT.store(SAMPLES, Ordering::Release);
                VERIFY_MISSES.store(misses, Ordering::Release);
                VERIFY_DONE.store(true, Ordering::Release);
            },
            "rt-verify-sleeper".into(),
            ax_task::default_task_stack_size(),
        );
        ax_task::spawn_task_with(task, |task| task.set_sched_priority(30))
    };

    let mut observed = false;
    for _ in 0..2_000_000 {
        if VERIFY_DONE.load(Ordering::Acquire) {
            observed = true;
            break;
        }
        thread::yield_now();
    }
    VERIFY_STOP.store(true, Ordering::Release);
    load.join();
    sleeper.join();

    if !observed {
        return Err("realtime verification: sleeper did not finish within the timeout");
    }

    let count = VERIFY_COUNT.load(Ordering::Acquire);
    let min = VERIFY_MIN_NS.load(Ordering::Acquire);
    let max = VERIFY_MAX_NS.load(Ordering::Acquire);
    let sum = VERIFY_SUM_NS.load(Ordering::Acquire);
    let misses = VERIFY_MISSES.load(Ordering::Acquire);
    let avg = sum / count.max(1);
    let jitter = max.saturating_sub(min);
    std::println!(
        "rt-verify: samples={count} period={}ms latency min/avg/max={min}/{avg}/{max} ns \
         jitter={jitter} ns deadline-misses={misses}/{count}",
        PERIOD.as_millis()
    );
    Ok(())
}
