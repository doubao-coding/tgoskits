use alloc::sync::Arc;
use core::sync::atomic::{AtomicIsize, Ordering};

macro_rules! def_test_sched {
    ($name:ident, $scheduler:ty, $task:ty) => {
        mod $name {
            use alloc::sync::Arc;

            use crate::*;

            #[test]
            fn test_sched() {
                const NUM_TASKS: usize = 11;

                let mut scheduler = <$scheduler>::new();
                for i in 0..NUM_TASKS {
                    scheduler.add_task(Arc::new(<$task>::new(i)));
                }

                for i in 0..NUM_TASKS * 10 - 1 {
                    let next = scheduler.pick_next_task().unwrap();
                    assert_eq!(*next.inner(), i % NUM_TASKS);
                    // pass a tick to ensure the order of tasks
                    scheduler.task_tick(&next);
                    scheduler.put_prev_task(next, false);
                }

                let mut n = 0;
                while scheduler.pick_next_task().is_some() {
                    n += 1;
                }
                assert_eq!(n, NUM_TASKS);
            }

            #[test]
            fn test_len() {
                const NUM_TASKS: usize = 7;
                let mut scheduler = <$scheduler>::new();
                assert_eq!(scheduler.len(), 0);
                assert!(scheduler.is_empty());
                let mut tasks = Vec::new();
                for i in 0..NUM_TASKS {
                    let t = Arc::new(<$task>::new(i));
                    tasks.push(t.clone());
                    scheduler.add_task(t);
                    assert_eq!(scheduler.len(), i + 1);
                }
                assert!(!scheduler.is_empty());
                for k in 0..NUM_TASKS {
                    assert_eq!(scheduler.len(), NUM_TASKS - k);
                    let _ = scheduler.pick_next_task().unwrap();
                }
                assert_eq!(scheduler.len(), 0);
                assert!(scheduler.pick_next_task().is_none());
                assert_eq!(scheduler.len(), 0);
                let mut scheduler = <$scheduler>::new();
                for t in &tasks {
                    scheduler.add_task(t.clone());
                }
                assert_eq!(scheduler.len(), NUM_TASKS);
                assert!(scheduler.remove_task(&tasks[3]).is_some());
                assert_eq!(scheduler.len(), NUM_TASKS - 1);
            }

            #[test]
            fn test_steal() {
                let mut scheduler = <$scheduler>::new();
                assert!(scheduler.pick_stealable_task(|_| true).is_none());
                for i in 0..5 {
                    scheduler.add_task(Arc::new(<$task>::new(i)));
                }
                assert!(scheduler.pick_stealable_task(|_| false).is_none());
                assert_eq!(scheduler.len(), 5);
                let stolen = scheduler.pick_stealable_task(|v| *v == 3).unwrap();
                assert_eq!(*stolen.inner(), 3);
                assert_eq!(scheduler.len(), 4);
                assert!(scheduler.pick_stealable_task(|v| *v == 3).is_none());
            }

            #[test]
            fn bench_yield() {
                const NUM_TASKS: usize = 1_000_000;
                const COUNT: usize = NUM_TASKS * 3;

                let mut scheduler = <$scheduler>::new();
                for i in 0..NUM_TASKS {
                    scheduler.add_task(Arc::new(<$task>::new(i)));
                }

                let t0 = std::time::Instant::now();
                for _ in 0..COUNT {
                    let next = scheduler.pick_next_task().unwrap();
                    scheduler.put_prev_task(next, false);
                }
                let t1 = std::time::Instant::now();
                println!(
                    "  {}: task yield speed: {:?}/task",
                    stringify!($scheduler),
                    (t1 - t0) / (COUNT as u32)
                );
            }

            #[test]
            fn bench_remove() {
                const NUM_TASKS: usize = 10_000;

                let mut scheduler = <$scheduler>::new();
                let mut tasks = Vec::new();
                for i in 0..NUM_TASKS {
                    let t = Arc::new(<$task>::new(i));
                    tasks.push(t.clone());
                    scheduler.add_task(t);
                }

                let t0 = std::time::Instant::now();
                for i in (0..NUM_TASKS).rev() {
                    let t = scheduler.remove_task(&tasks[i]).unwrap();
                    assert_eq!(*t.inner(), i);
                }
                let t1 = std::time::Instant::now();
                println!(
                    "  {}: task remove speed: {:?}/task",
                    stringify!($scheduler),
                    (t1 - t0) / (NUM_TASKS as u32)
                );
            }
        }
    };
}

def_test_sched!(fifo, FifoScheduler::<usize>, FifoTask::<usize>);
def_test_sched!(rr, RRScheduler::<usize, 5>, RRTask::<usize, 5>);
def_test_sched!(cfs, CFScheduler::<usize>, CFSTask::<usize>);

#[test]
fn rr_preempt_preserves_slice_but_forced_reschedule_rotates() {
    use alloc::sync::Arc;

    use crate::{BaseScheduler, RRScheduler, RRTask};

    let mut scheduler = RRScheduler::<usize, 5>::new();
    let current = Arc::new(RRTask::<usize, 5>::new(0));
    let remote = Arc::new(RRTask::<usize, 5>::new(1));
    scheduler.add_task(remote.clone());
    scheduler.put_prev_task(current.clone(), true);
    assert_eq!(
        *scheduler.pick_next_task().unwrap().inner(),
        0,
        "ordinary RR preemption preserves the current task's remaining slice",
    );

    let mut scheduler = RRScheduler::<usize, 5>::new();
    let current = Arc::new(RRTask::<usize, 5>::new(0));
    let remote = Arc::new(RRTask::<usize, 5>::new(1));
    scheduler.add_task(remote);
    scheduler.put_prev_task(current, false);
    assert_eq!(
        *scheduler.pick_next_task().unwrap().inner(),
        1,
        "forced remote reschedule must rotate the current task behind queued work",
    );
}

/// A test task that exposes a configurable realtime priority.
struct RtTestTask {
    id: usize,
    priority: AtomicIsize,
}

impl RtTestTask {
    fn new_task(id: usize, priority: isize) -> Arc<crate::RtFifoTask<Self>> {
        Arc::new(crate::RtFifoTask::new(Self {
            id,
            priority: AtomicIsize::new(priority),
        }))
    }
}

impl crate::RtPriority for RtTestTask {
    fn rt_priority(&self) -> isize {
        self.priority.load(Ordering::Acquire)
    }

    fn set_rt_priority(&self, priority: isize) -> bool {
        self.priority.store(priority, Ordering::Release);
        true
    }
}

#[test]
fn rt_fifo_picks_higher_priority_before_fifo_order() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    // Lower priority is enqueued first.
    scheduler.add_task(RtTestTask::new_task(0, 1));
    scheduler.add_task(RtTestTask::new_task(1, 10));

    let next = scheduler.pick_next_task().unwrap();
    assert_eq!(
        next.inner().id,
        1,
        "higher priority task must run before the earlier enqueued lower priority task",
    );
}

#[test]
fn rt_fifo_preserves_fifo_order_within_same_priority() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    scheduler.add_task(RtTestTask::new_task(0, 5));
    scheduler.add_task(RtTestTask::new_task(1, 5));
    scheduler.add_task(RtTestTask::new_task(2, 5));

    assert_eq!(scheduler.pick_next_task().unwrap().inner().id, 0);
    assert_eq!(scheduler.pick_next_task().unwrap().inner().id, 1);
    assert_eq!(scheduler.pick_next_task().unwrap().inner().id, 2);
}

#[test]
fn rt_fifo_tick_preempts_only_for_higher_priority_ready_task() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    let current = RtTestTask::new_task(0, 5);
    // Equal priority ready task: no preemption for realtime (priority > 0) tasks.
    scheduler.add_task(RtTestTask::new_task(1, 5));
    assert!(!scheduler.task_tick(&current));

    // Higher priority ready task: must request preemption.
    scheduler.add_task(RtTestTask::new_task(2, 10));
    assert!(
        scheduler.task_tick(&current),
        "a higher priority ready task must request a reschedule on tick",
    );
}

#[test]
fn rt_fifo_tick_rotates_default_priority_runtime_tasks() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    // Default priority (<= 0) tasks yield to an equal-priority ready task so a
    // runtime pool of default-priority work makes progress.
    let current = RtTestTask::new_task(0, 0);
    scheduler.add_task(RtTestTask::new_task(1, 0));
    assert!(
        scheduler.task_tick(&current),
        "default-priority tasks must rotate on tick",
    );
}

#[test]
fn rt_fifo_tick_does_not_rotate_equal_realtime_priority_tasks() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    let current = RtTestTask::new_task(0, 7);
    scheduler.add_task(RtTestTask::new_task(1, 7));
    assert!(
        !scheduler.task_tick(&current),
        "equal realtime-priority tasks must not be rotated by a tick",
    );
}

#[test]
fn rt_fifo_set_priority_reorders_ready_task() {
    use crate::{BaseScheduler, RtFifoScheduler};

    let mut scheduler = RtFifoScheduler::<RtTestTask>::new();
    let low = RtTestTask::new_task(0, 1);
    let high = RtTestTask::new_task(1, 1);
    scheduler.add_task(low.clone());
    scheduler.add_task(high.clone());

    // Boost `low` above `high` while it is still queued.
    assert!(scheduler.set_priority(&low, 20));
    assert_eq!(
        scheduler.pick_next_task().unwrap().inner().id,
        0,
        "priority change must immediately reorder the ready queue",
    );
}
