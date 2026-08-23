use alloc::{collections::BTreeMap, sync::Arc};
use core::{
    cmp,
    ops::Deref,
    sync::atomic::{AtomicIsize, Ordering},
};

use crate::BaseScheduler;

/// The realtime priority capability a schedulable entity must expose so that
/// [`RtFifoScheduler`] can order runnable tasks.
///
/// This is the minimal capability boundary between `ax-sched` and the concrete
/// task type: the scheduler only reads and sets the effective realtime priority
/// and never depends on internal task fields. `rt_priority` returns the
/// *effective* priority (including any temporary boost such as mutex priority
/// donation), while `set_rt_priority` writes the *base* priority.
pub trait RtPriority {
    /// Returns the effective realtime priority used to order the ready queue.
    fn rt_priority(&self) -> isize;

    /// Sets the base realtime priority. Returns `false` if the value is out of
    /// the supported range and the priority was left unchanged.
    fn set_rt_priority(&self, priority: isize) -> bool;
}

/// A task wrapper for [`RtFifoScheduler`].
///
/// It carries the inner task struct and the enqueue order used to keep FIFO
/// order among tasks with the same priority.
pub struct RtFifoTask<T> {
    inner: T,
    enqueue_order: AtomicIsize,
}

impl<T> RtFifoTask<T> {
    /// Creates a new [`RtFifoTask`] from the inner task struct.
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            enqueue_order: AtomicIsize::new(0),
        }
    }

    fn set_enqueue_order(&self, order: isize) {
        self.enqueue_order.store(order, Ordering::Release);
    }

    /// Returns a reference to the inner task struct.
    pub const fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T> Deref for RtFifoTask<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A realtime FIFO scheduler ordered by effective priority.
///
/// The highest priority runnable task is selected first. Tasks with the same
/// priority keep FIFO order and are **not** rotated by timer ticks: a tick
/// requests a reschedule only when a ready task with a strictly higher
/// priority than the current task exists, or when the current task runs at the
/// default priority (`<= 0`) and an equal-priority ready task is waiting.
pub struct RtFifoScheduler<T> {
    ready_queue: BTreeMap<(cmp::Reverse<isize>, isize), Arc<RtFifoTask<T>>>,
    enqueue_order: AtomicIsize,
}

impl<T: RtPriority> RtFifoScheduler<T> {
    /// Creates a new empty [`RtFifoScheduler`].
    pub const fn new() -> Self {
        Self {
            ready_queue: BTreeMap::new(),
            enqueue_order: AtomicIsize::new(0),
        }
    }

    /// Returns the name of the scheduler.
    pub fn scheduler_name() -> &'static str {
        "Realtime FIFO"
    }

    /// Returns the number of ready tasks currently in the ready queue.
    pub fn len(&self) -> usize {
        self.ready_queue.len()
    }

    /// Returns whether the ready queue is empty.
    pub fn is_empty(&self) -> bool {
        self.ready_queue.is_empty()
    }

    /// Removes and returns the highest-priority ready task for which `pred`
    /// holds. `None` if no task matches.
    pub fn pick_stealable_task(
        &mut self,
        mut pred: impl FnMut(&T) -> bool,
    ) -> Option<Arc<RtFifoTask<T>>> {
        let key = self
            .ready_queue
            .iter()
            .find(|(_, task)| pred(task.inner()))
            .map(|(key, _)| *key)?;
        self.ready_queue.remove(&key)
    }

    fn enqueue_task(&mut self, task: Arc<RtFifoTask<T>>) {
        let order = self.enqueue_order.fetch_add(1, Ordering::AcqRel);
        task.set_enqueue_order(order);
        self.ready_queue
            .insert((cmp::Reverse(task.rt_priority()), order), task);
    }
}

impl<T: RtPriority> BaseScheduler for RtFifoScheduler<T> {
    type SchedItem = Arc<RtFifoTask<T>>;

    fn init(&mut self) {}

    fn add_task(&mut self, task: Self::SchedItem) {
        self.enqueue_task(task);
    }

    fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        // Donation or `set_priority` may have changed the queued task's
        // effective priority, so the old BTreeMap key is unreliable. Find the
        // entry by pointer identity instead.
        let key = self
            .ready_queue
            .iter()
            .find(|(_, queued)| Arc::ptr_eq(queued, task))
            .map(|(key, _)| *key)?;
        self.ready_queue.remove(&key)
    }

    fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
        self.ready_queue.pop_first().map(|(_, task)| task)
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, _preempt: bool) {
        self.enqueue_task(prev);
    }

    fn task_tick(&mut self, current: &Self::SchedItem) -> bool {
        let Some(((priority, _), _)) = self.ready_queue.first_key_value() else {
            return false;
        };
        let ready_priority = priority.0;
        let current_priority = current.rt_priority();
        ready_priority > current_priority
            || (current_priority <= 0 && ready_priority == current_priority)
    }

    fn set_priority(&mut self, task: &Self::SchedItem, prio: isize) -> bool {
        // Remove first so the old key (stale priority) does not shadow the
        // re-insertion when `set_rt_priority` changes the effective priority.
        let queued = self.remove_task(task);
        if !task.set_rt_priority(prio) {
            if let Some(task) = queued {
                self.enqueue_task(task);
            }
            return false;
        }
        if let Some(task) = queued {
            self.enqueue_task(task);
        }
        true
    }
}

impl<T: RtPriority> Default for RtFifoScheduler<T> {
    fn default() -> Self {
        Self::new()
    }
}
