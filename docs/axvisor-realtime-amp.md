# Axvisor realtime CPU partitioning

## Problem and goal

On a four-core target, Axvisor must run StarryOS vCPU tasks on three physical
CPUs while one physical CPU runs host realtime tasks. Realtime task creators
must not know the selected CPU or construct a CPU mask. A build with no
realtime workload must retain all CPUs for ordinary work.

Success means that the build selects zero or one realtime CPU, ordinary and
vCPU tasks cannot execute there, realtime tasks are pinned there before their
first enqueue, and invalid topology fails during scheduler initialization.

## Configuration and invariants

`REALTIME_CPU_ID` is parsed by the `ax-task` build script. `-1` (the default)
means disabled; a nonnegative integer selects that logical CPU. Other negative
values and IDs outside the build-time `SMP` capacity are build errors. At boot,
the selected ID must be online and must not be the primary CPU.

The generated runtime representation is `Option<usize>`; the `-1` sentinel is
confined to the build boundary. The ordinary CPU mask is the online CPU set
minus the realtime CPU. VM configuration must reject a physical CPU set that
intersects the realtime CPU rather than silently remapping it.

## Scheduling and API

All CPUs initialize `axtask`, preserving the existing SMP, interrupt and IPI
startup contracts. With `sched-rt-fifo`, positive priorities are strict FIFO
realtime priorities. Priority zero is ordinary work and rotates on timer ticks.

`spawn_realtime(entry, name, stack_size, priority)` obtains the configured CPU
internally, installs its one-bit affinity and positive priority before the task
is registered or enqueued, and returns `RealtimeDisabled` or `InvalidPriority`
for caller-correctable errors. It deliberately has no CPU-mask parameter.

## Isolation boundary

The first stage provides scheduler/placement isolation, not full temporal or
memory isolation. Device interrupts, global locks, shared cache/memory buses,
firmware interrupts and console output can still add jitter. IRQ routing and
device ownership must be audited per platform before claiming hard realtime.

## Alternatives

An independent executor on the realtime CPU offers a smaller runtime surface,
but conflicts with realtime work that depends on `axtask` services. Dynamic CPU
borrowing improves utilization but weakens the static isolation contract. This
design therefore keeps a restricted `axtask` run queue and static ownership.

## Validation

Unit tests cover configuration parsing and RT-FIFO ordering/preemption. The
checked-in QEMU smoke test uses four CPUs, reserves CPU 3, and runs the host
realtime workload without a guest. It reports warmup and measured sample counts,
maximum and percentile latency, and missed deadlines. Physical-board validation
remains required for hard-realtime claims because QEMU cannot model interrupt
and memory-system interference.

The QEMU AMP manifest is under
`test-suit/axvisor/normal/qemu-amp/host-rt`. Run it with:

```sh
REALTIME_CPU_ID=3 cargo xtask axvisor test qemu \
  --arch aarch64 -g normal -c qemu-amp/host-rt
```

This validation deliberately has no external guest-image dependency. Guest
coexistence and FreeRTOS comparison remain separate follow-up validation work.

## Rollback

Building with `REALTIME_CPU_ID=-1` disables realtime task creation and restores
the full ordinary CPU mask. Disabling `sched-rt-fifo` restores the prior global
scheduler selection.
