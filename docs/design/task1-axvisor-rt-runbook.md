# 任务一：Axvisor 实时性改造与验证 runbook

本文档说明如何用本分支的实时性改造作为底座完成赛题任务一（实时性改造与验证），覆盖：
关键路径改造、多核 Linux 客户机启动配置、实时性验证流程、可复现交付。**RTOS 基线
（任务一第 4 点）不在本 runbook 范围内。**

## 1. 改造了哪些关键路径（底座）

改造在 ArceOS 调度器层（`os/arceos/`），Axvisor 经 `axstd` 直接受益（Axvisor 的宿主
任务/锁/定时器/中断就是 `ax_task`/`ax_sync`/`ax_hal`）。覆盖任务一第 1 点列出的全部
关键路径：

| 关键路径 | 改造 | 代码位置 |
|---|---|---|
| 调度 | `RtFifoScheduler`：按有效优先级选任务，同级 FIFO | `components/axsched/src/rt_fifo.rs` |
| 抢占 | tick 在更高优先级 ready 时请求重调度；唤醒在 safe-point 抢占 | `axtask/src/run_queue.rs`、`api.rs` |
| 定时器 | deferred softirq：硬 IRQ 只做最小工作，到期唤醒移到高优先级 softirq 任务 | `axtask/src/timers.rs` |
| 中断路径 | 硬 IRQ 关闭时间收敛（wakeup 不在硬中断同步执行） | `axtask/src/timers.rs`、`api.rs` |
| CPU 亲和性 | `RT_CPUMASK` 编译期配置 + `spawn_rt_task` 把 RT 任务钉到 RT 核；SMP 下跨核唤醒走 force-IPI | `axtask/build.rs`、`api.rs`、`run_queue.rs` |
| 锁临界区 | mutex 优先级继承（捐赠链 + 多锁重算），消除优先级反转 | `axtask/src/sync/mutex/mod.rs`、`bridge.rs`、`task.rs` |
| 后台任务 | gc/serial0-maint/timer-softirq 在 `sched-rt-fifo` 下按合适优先级运行 | `axtask/src/run_queue.rs`、`axruntime/src/serial/mod.rs` |

默认关闭（feature-gated `sched-rt-fifo`），不影响现有 Axvisor/ArceOS/StarryOS 配置。

## 2. 把改造接到 Axvisor（Step 1，已完成）

Axvisor 新增 `sched-rt-fifo` feature（透传 `ax-std/sched-rt-fifo`）：

```toml
# os/axvisor/Cargo.toml
sched-rt-fifo = ["ax-std/sched-rt-fifo"]
```

RT 板级配置 `os/axvisor/configs/board/qemu-aarch64-rt.toml`：4 物理核（SMP=4），
`pCPU3` 预留为实时域（`RT_CPUMASK=3`），2-vCPU Linux 客户机钉到 `pCPU0..1`。

验证接线（本机可跑）：
```bash
source /tmp/rt-env.sh   # 见下"可复现环境"
cargo xtask axvisor build -c qemu-aarch64-rt
# 产物：target/aarch64-unknown-none-softfloat/release/axvisor.bin（带 sched-rt-fifo + RT_CPUMASK=3）
```
本分支已验证该构建通过（release 编译成功，axvisor 带 `ax-std/sched-rt-fifo`）。

## 3. 多核 Linux 客户机启动配置（任务一第 2 点）

VM 配置 `os/axvisor/configs/vms/qemu/aarch64/linux-smp2.toml`（基于 `linux-smp1.toml`
扩为 2 vCPU）。绑定/内存/设备/中断/启动参数说明：

| 项 | 值 | 说明 |
|---|---|---|
| vCPU 数 | 2 | 满足"不少于 2 个 vCPU" |
| vCPU→物理核绑定 | vCPU0→pCPU0, vCPU1→pCPU1 | `phys_cpu_ids = [0, 1]`；`pCPU3` 是 RT 核，vCPU 永不落到其上 |
| 内存 | 1G，`0x8000_0000`，identity-mapped | `memory_regions` |
| 内核镜像 | `/guest/linux/linux-qemu-smp2`（fs 加载） | **运行时资产**，需自行放入 Axvisor 文件系统（见下） |
| 入口/加载地址 | `0x8020_0000` | `entry_point`/`kernel_load_addr` |
| DTB | 加载到 `0x8000_0000` | `dtb_load_addr` |
| 设备 | 无直通（`passthrough = []`），用虚拟平台设备 | `[devices]` |
| 中断路由 | vCPU 中断走虚拟 GIC；非 RT 外部 IRQ 不路由到 `pCPU3`（RT 核只处理本地 RT timer/RT 设备 IRQ/允许的 doorbell） | 由 Axvisor IRQ framework + `RT_CPUMASK` 边界约束 |
| 启动参数 | `cargo xtask axvisor qemu -c qemu-aarch64-rt` | 见"可复现" |

**Linux 客户机镜像**：本仓库不携带 Linux 镜像（运行时资产）。需准备一个 2-vCPU
aarch64 Linux `Image`（如 `Image-smp2`），放入 Axvisor 文件系统 `/guest/linux/linux-qemu-smp2`。
可用主线 kernel + `defconfig`（`CONFIG_NR_CPUS=2`）或现成 aarch64 QEMU 镜像。

## 4. 实时性验证流程（任务一第 3 点）

验证基准 `test-suit/arceos/rust/src/task/rt_verify.rs::run_realtime_verification`：
高优先级周期任务（`sleep(2ms)` × 200 次）在 CPU-bound 负载下，统计唤醒延迟
min/avg/max、抖动（max-min）、deadline miss（>10ms）。同一份代码在 `sched-rr`（基线）
和 `sched-rt-fifo`（改造后）下跑，做前后对比。

### 4.1 ArceOS 层（底座）实测数据（aarch64 QEMU，单核）

| 指标 | sched-rr（基线） | sched-rt-fifo（改造后） |
|---|---|---|
| 唤醒延迟 min/avg/max | 47.8 / 48.0 / 48.2 ms | 36 / 61 / 135 μs |
| 抖动 (max-min) | 411 μs | 99 μs |
| deadline miss (>10ms) | **200/200 (100%)** | **0/200** |

平均延迟低 ~790×，最大延迟低 ~355×，deadline miss 从 100%→0。

### 4.2 跑法

```bash
source /tmp/rt-env.sh
cargo xtask arceos test qemu --test-case sched-rr     --target aarch64-unknown-none-softfloat
cargo xtask arceos test qemu --test-case sched-rt-fifo --target aarch64-unknown-none-softfloat
# 看输出 "rt-verify: samples=200 period=2ms latency min/avg/max=... jitter=... deadline-misses=..." 行
```

另有 `pi_latency::run_wakeup_latency_benchmark` 单次唤醒延迟（rt-fifo ~0.65ms vs rr ~45ms）。

### 4.3 在改造后 Axvisor + Linux 客户机下跑（任务一要求的环境）

底座验证（4.1）在裸 ArceOS 上。任务一第 3 点要求"改造后 AxVisor + CPU 负载"下测量。
方法：在 Axvisor 宿主侧起一个 RT 周期任务（`spawn_rt_task` 跑上述采样逻辑），同时在
Linux 客户机里跑 CPU/网络/存储压力，测量宿主 RT 任务的延迟/抖动/miss。这需要 Linux
镜像（见 §3）+ Axvisor 实跑；本 runbook 提供配置与方法，实跑需补 Linux 镜像资产。

### 4.4 已覆盖 / 未覆盖指标

| 任务一第 3 点指标 | 状态 |
|---|---|
| 周期任务抖动 | ✅ `rt-verify` jitter |
| 调度延迟 | ✅ `rt-verify`/`pi_latency` wakeup latency |
| 最大延迟 | ✅ `rt-verify` max |
| 长时间稳定性 | ✅ `rt-verify` 200 周期 + deadline-miss 计数（可调大 SAMPLES 做长测） |
| 中断响应延迟 | ⚠️ 未测（需设备 IRQ 触发器，后续） |
| CPU 负载分布 | ⚠️ ArceOS 单核跑；多核负载分布需 Axvisor+guest 实跑 |

## 5. 可复现交付（任务一第 5 点）

### 5.1 环境

```bash
# /tmp/rt-env.sh
export PATH=/workspace/tools/hostdeps/root/usr/bin:/workspace/tools/qemu/bin:$PATH
export PKG_CONFIG_PATH=/workspace/tools/hostdeps/root/usr/lib/x86_64-linux-gnu/pkgconfig
export LD_LIBRARY_PATH=/workspace/tools/llvm/root/usr/lib/x86_64-linux-gnu:/workspace/tools/hostdeps/root/usr/lib/x86_64-linux-gnu
export LIBCLANG_PATH=/workspace/tools/llvm/root/usr/lib/llvm-19/lib
```

### 5.2 构建/启动/测试命令

```bash
# ArceOS 底座验证（rr 基线 vs rt-fifo 改造后）
cargo xtask arceos test qemu --test-case sched-rr      --target aarch64-unknown-none-softfloat
cargo xtask arceos test qemu --test-case sched-rt-fifo --target aarch64-unknown-none-softfloat
# SMP + RT 亲和
cargo xtask arceos test qemu --test-case sched-rt-fifo-smp  --target aarch64-unknown-none-softfloat
cargo xtask arceos test qemu --test-case sched-rt-fifo-iso  --target aarch64-unknown-none-softfloat

# Axvisor 带 RT 构建
cargo xtask axvisor build -c qemu-aarch64-rt
# Axvisor 带 RT 启动（需 Linux 镜像资产，见 §3）
cargo xtask axvisor qemu  -c qemu-aarch64-rt
```

### 5.3 分支/配置/测试脚本

- 分支：`xagent-rt-opt`（本地 commit，未推送）。
- 配置：`os/axvisor/configs/board/qemu-aarch64-rt.toml`、
  `os/axvisor/configs/vms/qemu/aarch64/linux-smp2.toml`、
  ArceOS test cases `test-suit/arceos/rust/cases/sched-rt-fifo{,-smp,-iso}/`。
- 测试脚本：`cargo xtask arceos test qemu --test-case ...`（见上）。
- 结果采集：`rt-verify` / `pi_latency` 输出行可直接 grep 采集。

### 5.4 未做项（明确边界）

- **RTOS 基线**（Zephyr/RT-Thread/FreeRTOS，任务一第 4 点）：按需求跳过。
- **中断响应延迟**：需设备 IRQ 触发器，后续。
- **Axvisor + Linux 客户机实跑**：配置与方法已就绪，缺 Linux 镜像运行时资产。
- **真机数**：QEMU 计时不代表真实硬件，需上板（RK3588 等）复测。
