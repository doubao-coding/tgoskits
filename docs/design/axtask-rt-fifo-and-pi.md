# ArceOS host-side realtime scheduling and priority inheritance

## 1. 问题与目标

ArceOS 默认 FIFO 调度器只表达“先入队先运行”，不表达“高优先级先运行”；
sleepable `Mutex` 只做互斥与阻塞唤醒，没有把 waiter 的优先级反馈给持锁 owner。
在需要确定性响应的场景下，这会导致两类失败：

1. 低优先级任务先入队后，高优先级任务无法抢占。
2. 高优先级任务等待低优先级 owner 持有的锁时，被中优先级任务间接阻塞
   （经典优先级反转）。

本设计**不绕过 host 调度器**：不引入独立的 RT 执行器、不预留专属核、不另起
mailbox。它在 host ArceOS 调度器与同步原语内部增加实时语义，让 host 自身具备
RT 能力。这与 `pengzechen/tgoskits@prepare-pr` 的 AMP 路线不同——那条路线
（`ax-rt` 执行器 + 预留核 + doorbell mailbox）被仓库维护者拒绝，因为它旁路了
host。本设计吸收 prepare-pr 中**已在 host 内**的部分（`RtFifoScheduler`、
优先级继承互斥量、设计文档骨架与测试模板），丢弃所有旁路与应用代码。

## 2. 分阶段实现

### 阶段 0：host 内 RT 调度 + 单 mutex 优先级继承（默认关闭，feature-gated）

新增 `sched-rt-fifo` feature（`axtask`/`axruntime`/`axstd` 透传），默认关闭，
不影响现有配置。

- `components/axsched/src/rt_fifo.rs`：`RtFifoScheduler` 用
  `BTreeMap<(Reverse(priority), enqueue_order), _>` 维护 ready queue，高优先级
  先选、同级 FIFO；`task_tick` 只在 ready queue 存在比当前任务更高优先级任务
  时请求重调度，不对同级 RT 任务做时间片轮转。`RtPriority` 是 `ax-sched` 与
  具体任务类型之间的最小能力边界，调度算法不依赖 `axtask` 内部字段。
- `axtask::task::TaskInner`：把 `sched_priority` 拆成 `base_sched_priority`、
  `effective_sched_priority`、`donated_sched_priority`（仅 `sched-rt-fifo`），
  并增加 `mutex_wait_owner_id` 用于捐赠链传播。`sched_priority()` 返回
  effective 值。`impl RtPriority for TaskInner` 以 effective 优先级接入调度器。
- `axtask::sync::mutex::RawMutex`：contended lock 路径调用
  `donate_owner_priority(owner_id)`，沿 `mutex_wait_owner_id` 链式捐赠
  （`donate_priority_chain`，以 `CPU_CAPACITY.max(32)` 为有界保护），
  并通过 `run_queue::requeue_task_after_priority_change` 重排 ready queue；
  unlock 时 `clear_current_priority_donation()` 恢复 owner 基础优先级。
  lockdep 桥接路径（`sync/bridge.rs`）镜像同一逻辑，保证启用 lockdep 不改变
  PI 语义。非 `sched-rt-fifo` 路径全部 no-op。
- SMP 限制：`#[cfg(all(sched-rt-fifo, smp))] compile_error!`，因为系统级
  “最高优先级先运行”需要跨核 push/pull 与远程抢占，当前未实现。

### 阶段 1：补 prepare-pr 留的两个洞

#### 1(b) priority-aware WaitQueue 唤醒（已实现）

`WaitQueue` 原为 `VecDeque<AxTaskRef>` + `pop_front`，唤醒顺序=入队顺序，与
优先级无关。改为 `pop_next_waiter_index`：在 `sched-rt-fifo` 下按
`sched_priority` 选最高优先级 waiter 唤醒，其它配置保持 FIFO。`notify_one`、
`notify_one_with`、`notify_all` 均走该选择器。`notify_one_with` 仍在校验
锁内调用 `func`，保留 lock handoff 的原子性不变式。

这一条同时喂给 mutex 与普通 `WaitQueue`：mutex unlock 时 `wake_one` 现在唤醒
最高优先级 waiter，配合 owner donation 形成“owner 先释放→最高优先级 waiter
先获锁”的闭环。

#### 1(a) 多锁 PI 重算（已实现并验证）

原 Phase 0 的 unlock 盲清 `donated_sched_priority`，在 owner 同时持有多个
mutex 时会误清另一个 mutex 的 donation。现已改为 per-mutex top-waiter 缓存
+ held-registry 重算，**不修改** axsync repr(C) `RawMutex` 布局、**不修改**
`MutexOps` 接口——全部局限在 axtask 内。

**实现**（live 路径是 axtask 内部 `sync/mutex/mod.rs` 的 `RawMutex`，即
`ax_runtime::sync` = `ax_task::sync::api`，不是 bridge；bridge 路径镜像同一逻辑）:

1. `TaskInner` 增加 `held_mutexes: SpinLock<Vec<(usize, i32)>>`（cfg
   `sched-rt-fifo`），每项 `(lock_key, top_waiter_priority)`。`lock_key` =
   `RawMutex` 实例地址（稳定，mutex 越过持锁期存活）。
2. 获取成功（`lock_after_prepare`/`try_lock` 的 `Ok` 臂）：
   `register_held_mutex(lock_key)` 压入 `(lock_key, i32::MIN)`。
3. waiter 阻塞（`donate_owner_priority(owner, lock_key)`）：捐赠前先对
   owner 的 held 条目 `bump_held_top_waiter(lock_key, waiter_prio)`
   （取较高值），再走 `donate_priority_chain` 传播。深链环节的 top_waiter
   在其自身阻塞时已 bump。
4. 释放（`unlock`）：`deregister_held_mutex_and_recompute(lock_key)` 移除
   本 mutex 条目，重算 `donated = max(剩余条目 top_waiter)`（空则 `i32::MIN`），
   `refresh_effective_sched_priority` + `requeue`。盲清被彻底替代。

**验证**：新增 `run_mutex_donation_survives_other_mutex_release_test`——low
持 A+B、high(30) 等 A、low 释放 B（无 waiter）后记录自身 effective priority，
断言为 30（A 的 donation 保留）。在 aarch64 QEMU 实测通过
（`sched-rt-fifo multi-mutex donation survives OK`），且全部既有 PI 测试不回归。

**已知边界**：waiter 超时/取消离开 WQ 时 top_waiter 可能残留偏高（单调升
不降），只导致 owner 短期 donation 偏高、不漏唤醒，下一次该 mutex release 的
recompute 读到的也是该值；可接受。

### 阶段 2：让 host 给出紧凑抢占保证（不绕过 host 的代价）

prepare-pr 用独立执行器轮询时间绕开 100Hz tick 与硬 IRQ 同步唤醒路径；既然
不绕过 host，host 自身必须给出等价保证。本阶段是高风险（改变 host 定时器
硬中断路径），全部 feature-gate 在 `sched-rt-fifo` 下，默认配置行为不变。

- **2a 可配置 tick**（已实现）：`axruntime/build.rs` 增加 `TICKS_PER_SEC` env
  出口 + 单测；RT 配置下可提高到 1000Hz+。默认仍 100Hz。实测
  `TICKS_PER_SEC=1000` 把生成常量从 100 改为 1000。
- **2b RT 唤醒走 force-IPI**（已实现并验证）：阶段 4 解禁 `sched-rt-fifo`+`smp`
  后，`add_task`/`unblock_task` 在 `sched-rt-fifo` 下走 `force_kick_remote_cpu`
  （非合并 IPI）而非 `kick_remote_cpu`，保证 RT 唤醒不被 IPI 合并延迟。aarch64
  `-smp 2` QEMU `sched-rt-fifo-smp` 用例验证跨核 spawn 唤醒（high-prio 任务 pin
  到 CPU1，从 CPU0 spawn → force-IPI → CPU1 运行）。
- **2c threaded IRQ / 软中断分离**（已实现并验证，`sched-rt-fifo` 默认路径）：把
  定时器到期唤醒工作（`check_callbacks` + `expireOne` 循环 + future）从硬中断推迟到
  一个 per-CPU 高优先级 `timer-softirq` 任务，硬中断只保留 `check_irq_callbacks` +
  `scheduler_timer_tick` + 重编程。`timers.rs` 新增 deferred 意图状态机
  （`AtomicI8`：0 无、1 无回调、2 带回调，CAS 合并、swap 抽干）+
  `WaitQueue` 唤醒 + 防丢失唤醒的 `wait_until` 条件等待。意图状态机有 host
  单测 `deferred_intent_coalesces_and_drains`。**实测**：softirq 在 aarch64 QEMU boot
  期间正确 drain/wait/wake 循环，sched-rt-fifo 全部 PI 测试通过。关键实现细节：
  `wait_for_deferred_timer_expiry` 必须用裸 `with_cpu_pin`（镜像 GC 任务的 wait
  路径），**不能**裹 `PreemptIrqSaveGuard`——多一层 guard 会使 preempt count 变 3，
  撞上 `blocked_resched` 的 `assert!(can_preempt(2))`，在 boot 早期静默崩溃。非
  `sched-rt-fifo` 路径行为字节等价。

### 阶段 3：host 内 RT CPU 亲和（已实现并验证）

这是对“预留核”思想的合法翻译——核仍是 host 调度器管理的核，不另起内核。
依赖阶段 4 的 SMP+RT 解禁。

**配置**：`axtask/build.rs` 增加 `RT_CPUMASK` env（逗号分隔 CPU 索引）→ build_info
`pub const RT_CPUMASK: &[usize]`。空 = 不隔离。

**RT 亲和**：`ax_task::rt_cpu_mask()` 返回 RT 核集合；`ax_task::spawn_rt_task`
把任务 `cpumask` 限到 RT 核。aarch64 `-smp 2` + `RT_CPUMASK=1` 的
`sched-rt-fifo-iso` 用例验证 `spawn_rt_task` 任务落到 CPU1（`rt=cpu1`）。

**未实现（后续）**：把*所有*默认任务（boot/系统/普通）排除出 RT 核需要
task-class-aware 默认 mask——直接改 `cpu_mask_full` 全局排除会破坏 SMP boot
（已实测卡死在 secondary bringup）。完整 host-internal 隔离 + `IrqAffinity`
路由 + tickless idle 留作后续。

### 阶段 4：SMP+RT 跨核调度（已实现并验证，最小版）

撤去 `#[cfg(all(sched-rt-fifo, smp))] compile_error!`。SMP 下用 per-CPU
`RtFifoScheduler` + 跨核唤醒走 force-IPI（见 2b）。**已知边界**：这是 per-CPU +
跨核唤醒抢占，**不**实现全局 push/pull 平衡，故不保证“系统级最高优先级先跑”——
一个 CPU0 上的高优先级任务不会主动迁移到正在跑低优先级任务的 CPU1。更强保证
（全局拉模型 + 每核 `highest_ready_priority` 原子缓存）是后续工作。

**验证**：`sched-rt-fifo-smp`（`-smp 2`）跨核 spawn 唤醒 PASS。host-test 单 CPU
模型无法覆盖跨核，端到端由 QEMU 承担。

## 3. 测试与验证

| 声明 | 验证层级 | 命令 | 通过条件 |
|------|----------|------|----------|
| `RtFifoScheduler` 优先级排序与 tick 抢占判定 | `ax-sched` host 单测 | `cargo test -p ax-sched` | 22 个 RT FIFO 单测通过 |
| PI mutex + `sched-rt-fifo` 接入不破坏现有 mutex/sync 行为 | `ax-task` host 单测 | `cargo test -p ax-task --features host-test,multitask,sched-rt-fifo --lib` | 52 单测通过 |
| RT FIFO 调度 + mutex PI 端到端（优先级反转/ABC 链/try_lock 不捐赠/最高 waiter） | aarch64 QEMU 集成 | `cargo xtask arceos test qemu --test-case sched-rt-fifo --target aarch64-unknown-none-softfloat` | **PASS**：全部 PI 用例输出成功标记 + `ArceOS test suite run OK!` |
| 软中断 deferred 意图状态机（合并/抽干/不丢回调） | `ax-task` host 单测 | `cargo test -p ax-task --features host-test,multitask,sched-rt-fifo --lib` | `deferred_intent_coalesces_and_drains` 通过（53 单测） |
| priority-aware WaitQueue 唤醒最高优先级 waiter | aarch64 QEMU 集成 | 同 `sched-rt-fifo` case（`run_mutex_uses_highest_waiter_priority_test`） | **PASS**：highest-waiter donation OK |
| 软中断 live IRQ 路径（到期唤醒走 softirq、boot 期间 drain/wait/wake 循环） | aarch64 QEMU 集成 | `cargo xtask arceos test qemu --test-case sched-rt-fifo --target aarch64-unknown-none-softfloat` | **PASS**：boot 期间 softirq 正确循环，全部 PI 测试通过 |
| 多锁 PI 重算（释放无 waiter 的 mutex 不清另一个的 donation） | aarch64 QEMU 集成 | 同 `sched-rt-fifo` case（`run_mutex_donation_survives_other_mutex_release_test`） | **PASS**：multi-mutex donation survives OK |
| SMP+RT 跨核唤醒走 force-IPI（2b） | aarch64 QEMU 集成 | `cargo xtask arceos test qemu --test-case sched-rt-fifo-smp --target aarch64-unknown-none-softfloat` | **PASS**：cross-CPU wake OK（high-prio 任务 pin 到 CPU1、从 CPU0 spawn → force-IPI → CPU1 运行） |
| host 内 RT CPU 亲和（`RT_CPUMASK` + `spawn_rt_task`） | aarch64 QEMU 集成 | `cargo xtask arceos test qemu --test-case sched-rt-fifo-iso --target aarch64-unknown-none-softfloat`（`RT_CPUMASK=1`） | **PASS**：RT affinity OK (rt=cpu1) |
| 不启用 `sched-rt-fifo` 时行为不变 | `ax-task` 默认配置 | `cargo clippy -p ax-task` + `cargo test -p ax-task --features host-test,multitask --lib` | 48 单测通过、无回归 |

已知限制：`ax-task` 的 `WaitQueue` doctest（`wait_queue.rs` 第 12 行）在
`host-test` + 任意抢占式调度器（`sched-rr` 与 `sched-rt-fifo`）下因 host-test
调度器模型限制而失败；这在 `sched-rr` 下本就存在，非本改动引入。RT 调度的端到端
验证由 QEMU 集成测试承担。

## 4. 非目标

- 不引入独立 RT 执行器或预留专属核（阶段 3 的 RT CPU 亲和仍在 host 调度器内）。
- 不实现 POSIX `PTHREAD_PRIO_INHERIT` 完整 ABI 语义。
- 不实现全局 push/pull 平衡（阶段 4 为 per-CPU + force-IPI 跨核唤醒，不保证系统级最高优先级先跑）。
- 不强制*所有*默认任务排除出 RT 核（完整 host-internal 隔离需 task-class-aware mask，后续工作）。
- 不实现 `IrqAffinity` 路由与 tickless idle（后续工作）。
- 不改变非 `sched-rt-fifo` 配置的 mutex 或调度行为。
