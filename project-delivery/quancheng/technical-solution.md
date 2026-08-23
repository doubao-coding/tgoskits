# 智能化工控中基于虚拟化的混合系统部署及联动实现技术方案

## 1. 项目概述

### 1.1 项目背景

本项目面向“智能化工控中基于虚拟化的混合系统部署及联动实现”这一赛题场景，核心是在同一硬件或仿真平台上同时承载智能侧客户机与隔离实时 CPU 上的控制任务，并通过 Axvisor 的 AMP 隔离方案实现资源隔离、稳定通信和联动控制。与传统将控制、智能和业务分散到多套设备的部署方式相比，该方案更强调在统一底座上完成系统整合，既要保证通用负载可运行，也要保证实时控制具备确定性，还要让 AI 推理真正进入控制闭环。

从赛题要求看，本次攻关不是单点功能开发，而是围绕三个相互衔接的任务展开：任务一解决实时性与隔离问题，任务二解决客户机间通信与协议问题，任务三在前两项能力基础上完成 AI 与控制联动的应用展示。因此，本方案不把三个任务写成并列功能，而是将其组织为“底座、链路、应用”三层递进关系。

### 1.2 赛题三项任务理解

任务一提供 Axvisor 自身的实时运行保障，包括实时 CPU 预留、智能侧 vCPU 与实时侧 CPU 隔离、中断与定时器路径处理、板级设备访问和调频归因优化，目标是让 Axvisor 在 AI guest、I/O 和实时控制任务并存时仍保持可接受的时延和稳定性。

任务二负责把智能侧与实时侧连接起来，建立可演进的控制通信链路，并在其上形成应用层协议、可靠性机制和自动化测试方法，使控制指令、状态回传和异常处理都有明确的消息语义。

任务三把前两项能力组合成实际应用，令智能侧客户机完成 AI 推理后，把结果送达 Axvisor 实时侧控制路径，由预留实时 CPU 上的控制任务执行动作并回传状态，形成可观测、可测量、可复现的闭环。

这种分工的意义在于，任务一和任务二属于基础能力建设，任务三属于能力落地应用。如果没有任务一，系统无法稳定运行，联动闭环的时延和抖动也无法满足验收要求；如果没有任务二，智能侧与控制侧之间缺少标准化、可验证的数据通道；如果没有任务三，前两项工作只能停留在底层实现，无法体现赛题要求的联动价值。

### 1.3 总体目标与验收口径

项目总体目标是构建一套面向工控场景的虚拟化混合系统方案，使 Linux/StarryOS 智能侧客户机与 Axvisor 预留实时 CPU 上的控制任务能够在同一平台上稳定协同工作，并完成 AI 驱动控制的端到端展示。对应验收口径包括：

- 系统能启动、能部署、能在多客户机环境下稳定运行。
- 客户机之间能可靠通信，通信行为可通过协议和测试数据复现。
- AI 推理到控制执行的链路可闭环，端到端时延可测。
- 源码、配置、PR、测试记录和文档齐备，能够支撑复现和评审。

### 1.4 三任务关系说明

三项任务之间的关系可以概括为“底座、链路、应用”三层结构。任务一是 Axvisor 实时性与隔离底座，优化虚拟化运行时自身在 AMP 场景下的关键路径；任务二是通信链路，提供跨执行域的数据交换和协议封装；任务三是应用层闭环，在前两者基础上实现 AI 与控制的联动演示。

从整体方案看，任务一和任务二分别形成系统底座和通信链路，任务三在二者之上形成端到端应用闭环。这样既保留每个任务的独立技术边界，也形成从底层能力到上层应用的递进关系。

## 2. 总体架构设计

![智能化工控虚拟化混合系统总体架构](assets/architecture.svg)

### 2.1 系统分层架构

系统整体划分为四层：硬件与仿真平台层、Axvisor 虚拟化与 AMP 隔离层、智能侧客户机与实时侧控制层、AI 联动应用层。

硬件与仿真平台层提供 QEMU 或开发板运行环境，包含 CPU、内存、定时器、中断控制器、网络设备、块设备以及可选 NPU/AI 加速资源。该层既用于本地快速复现，也用于板级验证。

Axvisor 虚拟化与 AMP 隔离层负责智能侧 VM 生命周期管理、vCPU 管理、地址空间隔离、虚拟设备管理、中断与定时器路径处理，并为实时控制预留独立 CPU。该层是任务一的主要承载位置，也是任务二和任务三能够运行在同一平台上的基础。

智能侧客户机与实时侧控制层包含 StarryOS/Linux 智能侧 guest 和 Axvisor 实时侧控制路径。智能侧客户机承载视觉或语音感知、模型推理、协议客户端和状态上报逻辑；实时侧控制路径运行在预留 CPU 上，承载 8ms 控制闭环、状态采集、动作执行和超时降级逻辑，不再额外启动一个控制侧 guest OS。

AI 联动应用层负责把推理结果转换为控制语义，并通过任务二提供的控制通道发送到实时侧。实时侧执行后返回状态、时间戳和异常码，形成端到端闭环。

```text
AI 联动应用层
  ├─ 智能侧：图像/传感输入 -> AI 推理 -> 控制指令生成
  └─ 控制侧：指令解析 -> 控制执行 -> 状态回传

智能侧客户机与实时侧控制层
  ├─ Linux/StarryOS 智能侧客户机
  └─ Axvisor 预留实时 CPU 上的 RT 控制任务

Axvisor 虚拟化与 AMP 隔离层
  ├─ 智能侧 VM/vCPU/地址空间/虚拟设备/中断与定时器
  └─ 实时 CPU 预留、板级外设访问、资源分配与共享测试入口

硬件与仿真平台层
  ├─ QEMU 多架构验证
  └─ 开发板/NPU/网络/块设备等外设资源
```

### 2.2 客户机角色划分

智能侧客户机负责计算密集型或系统服务型任务，典型职责包括输入数据采集、AI 模型加载、推理结果生成、控制策略封装、运行日志记录和异常上报。该客户机允许运行较完整的软件栈，重点强调功能完备性和模型运行能力。

实时侧控制路径负责实时性要求更高的控制任务，典型职责包括接收智能侧指令、执行控制动作、采集控制状态、返回执行结果和处理超时降级。该路径运行在隔离出的 CPU 上，由 Axvisor 直接承载实时任务，强调确定性、低抖动和可隔离运行，不依赖控制侧 guest OS 调度。

Axvisor 作为统一底座，负责把智能侧 guest 与实时侧 CPU 放在同一平台上协同运行，并通过资源配置、中断路径控制、实时 CPU 预留和设备访问边界降低相互干扰。

### 2.3 数据流与控制流

端到端链路从智能侧输入开始，经 AI 推理得到识别结果或决策结果，再由协议模块转换为控制消息。控制消息通过 RT mailbox、console 原型或后续结构化控制通道发送到 Axvisor 实时侧。实时侧控制任务执行控制动作，并将执行状态、错误码和时间戳返回智能侧。智能侧根据回传结果记录闭环状态，也可以在失败、超时或置信度不足时触发降级策略。

```text
输入数据
  -> 智能侧客户机 AI 推理
  -> 应用层协议封装
  -> 控制通道
  -> Axvisor 实时侧接收
  -> 控制执行
  -> 状态回传
  -> 智能侧闭环记录与策略调整
```

### 2.4 三任务依赖关系

任务一是所有任务的运行基础，直接决定 Axvisor 能否在智能侧 guest 与实时侧任务并存时保持稳定，资源能否隔离，关键路径能否保持低抖动。任务二依赖任务一提供的隔离运行环境打通通信链路。任务三依赖任务一和任务二，只有当 Axvisor 实时路径稳定且通信协议可靠后，AI 推理结果才可以进入控制闭环。

因此，验收顺序也应保持一致：先验证虚拟化底座和隔离能力，再验证客户机通信链路，最后验证 AI 联动应用闭环。

## 3. 任务一：Axvisor 实时性与隔离基础设计

### 3.1 任务目标

任务一目标是优化 Axvisor 自身的实时性，使同一平台上的智能侧 guest、虚拟设备 I/O 和实时侧控制任务能够稳定共存。该任务不只关注“能启动”，还关注 Axvisor 在并发负载下的实时 CPU 预留、智能侧 vCPU 与预留物理 CPU 的隔离关系、中断与定时器路径、板级设备访问和调频策略对 8ms 控制闭环的影响。

本次任务一以 Axvisor 运行时为主线分层完成。#2160 位于 Axvisor 和 runtime 边界，解决实时 CPU 预留和 AMP 隔离运行的问题，是任务一的核心；#2163、#2164 和 #2165 从 Axvisor 的块设备启动、板级 SD 主机和 RK3588 调频归因补齐底座能力，降低 guest 加载、I/O 和频率管理对实时路径的扰动；#2161 和 #2162 提供预留 CPU 上实时任务的调度与锁等待支撑，但不再把任务一表述为“启动一个控制侧 guest OS 并优化其内部调度”。

| PR | 技术层次 | 核心目标 | 依赖关系 |
| --- | --- | --- | --- |
| [#2160](https://github.com/rcore-os/tgoskits/pull/2160) | Axvisor 实时 CPU 预留 | 为 Axvisor 保留实时 CPU，降低实时控制路径被智能侧 guest 和普通负载干扰的风险 | 任务一核心 |
| [#2163](https://github.com/rcore-os/tgoskits/pull/2163) | Axvisor guest 启动与块设备路径 | 增加 IRQ 驱动 virtio-blk 和 Starry guest smoke，降低启动与块 I/O 路径不确定性 | 底座支撑 |
| [#2164](https://github.com/rcore-os/tgoskits/pull/2164) | Axvisor 板级 SD 主机启用 | 使 OrangePi 5 Plus 上 Axvisor 能从 SD 文件系统加载 guest 镜像 | 板级支撑 |
| [#2165](https://github.com/rcore-os/tgoskits/pull/2165) | RK3588 调频归因修复 | 按 FDT CPU topology 归因 guest busy，避免实时相关 CPU 簇被错误降频 | 板级实时性支撑 |
| [#2161](https://github.com/rcore-os/tgoskits/pull/2161) | 实时任务调度支撑 | 增加单核 `sched-rt-fifo`，使预留 CPU 上高优先级任务先于低优先级任务运行 | 组件支撑 |
| [#2162](https://github.com/rcore-os/tgoskits/pull/2162) | 同步原语与优先级继承 | 在 RT FIFO 基础上为 mutex 添加 priority inheritance，降低锁等待导致的间接阻塞 | 组件支撑 |

### 3.2 智能侧 VM 与实时 CPU 配置方案

智能侧资源配置仍以 VM 配置文件为核心，描述 guest 的 CPU 数量、内存区域、入口地址、镜像路径、设备列表和可访问外设。实时侧不再建模为另一个控制侧 guest，而是由 Axvisor 识别并保留专用于控制路径的物理 CPU，直接承载 8ms 控制闭环和必要外设访问。#2160 在这一层补齐实时 CPU 预留能力，使 Axvisor 可以在 host 侧识别并保留专用于实时侧控制路径的 CPU 资源。

在 QEMU 验证阶段，资源配置用于验证智能侧 guest 启动、虚拟设备和基础运行。在板级验证阶段，资源配置与真实设备地址、中断号、内存布局以及实时侧可访问外设对应。实时 CPU 预留不是直接替代虚拟地址空间隔离，而是在已有 VM 内存和设备隔离之外增加 CPU 时间维度的隔离，避免智能侧计算密集负载长期占用实时控制关键路径所需的执行资源。

| 配置或模块 | 职责 | 对任务一的作用 |
| --- | --- | --- |
| `os/axvisor/src/realtime.rs` | 管理 Axvisor 侧实时 CPU 预留语义 | 明确哪些 CPU 可作为控制侧实时资源 |
| `os/arceos/modules/axruntime/build.rs` | 在构建阶段生成或传递运行时 CPU 信息 | 让 runtime 能识别实时 CPU 配置 |
| `virtualization/axvm/build.rs` | 为 axvm host glue 提供构建期配置输入 | 将实时 CPU 信息传递到虚拟化组件 |
| `virtualization/axvm/src/host/arceos.rs` | AxVM 与 ArceOS host 的适配层 | 让 VM/vCPU 管理能够消费 runtime 侧隔离信息 |

### 3.3 调度与关键路径优化方案

实时性设计重点关注预留实时 CPU 上控制任务的调度确定性。#2161 在 `components/axsched/src/rt_fifo.rs` 中新增 `RtFifoScheduler`，用 `RtPriority::rt_priority()` 获取任务有效优先级，并以 `(Reverse(priority), enqueue_order)` 维护 ready queue。高优先级任务总是先于低优先级任务被选中；同优先级任务仍保持 FIFO 入队顺序，符合控制任务常见的实时 FIFO 语义。

该调度能力通过 `sched-rt-fifo` feature 接入 `axtask` 和 `ax-std`，默认 FIFO、RR 和 CFS 路径不被替换。当前实现明确限定在 `SMP=1`，因为多核实时调度还需要跨 CPU push/pull、远程抢占和任务迁移协议才能保证系统级最高优先级先运行。比赛任务一中，它先作为预留实时 CPU 上单核实时任务的调度底座。

| 调度能力 | 代码锚点 | 验证方式 |
| --- | --- | --- |
| 高优先级优先 | `RtFifoScheduler::pick_next_task()` | `rt_fifo_picks_higher_priority_before_fifo_order` |
| 同优先级 FIFO | `enqueue_order` 和 ready queue key | `rt_fifo_preserves_fifo_order_within_same_priority` |
| ready task 改优先级后重排 | `RtFifoScheduler::set_priority()` | `rt_fifo_set_priority_reorders_ready_task` |
| 默认优先级轮转判定 | `RtFifoScheduler::task_tick()` | `rt_fifo_tick_rotates_default_priority_runtime_tasks` |
| ArceOS QEMU 集成 | `test-suit/arceos/rust/cases/sched-rt-fifo/` | `cargo xtask arceos test qemu --test-group rust --test-case sched-rt-fifo --target x86_64-unknown-none` |

### 3.4 中断、定时器与绑核设计

虚拟化环境下，中断、定时器和绑核路径是实时性的重要影响因素。#2160 先在 Axvisor 侧建立实时 CPU 预留边界，使实时控制任务可以与智能侧普通 vCPU 形成更清晰的资源隔离；#2161 再为实时任务提供 FIFO 调度，让控制任务被唤醒后能够按优先级运行。

在 x86 场景中，可结合 `components/x86_vlapic` 分析虚拟本地 APIC 定时器和中断注入路径；在 AArch64 场景中，可结合 `components/arm_vgic` 和板级配置分析虚拟中断控制器行为。绑核设计用于降低智能侧 vCPU 迁移带来的缓存扰动和调度不确定性；RT FIFO 则处理预留实时 CPU 上的任务级优先级选择。

```text
Axvisor 实时 CPU 预留
  -> 实时控制任务与智能侧 vCPU 隔离
  -> sched-rt-fifo 选择高优先级控制任务
  -> mutex PI 避免锁等待导致的优先级反转
  -> 实时控制关键路径获得更稳定的执行机会
```

### 3.5 隔离设计

隔离设计包括内存隔离、CPU 时间隔离、设备访问隔离和同步路径隔离。内存隔离通过虚拟地址空间和 VM 配置限定智能侧 guest 可访问范围；CPU 时间隔离通过 #2160 的实时 CPU 预留、vCPU 配置和绑核策略降低互相干扰；设备访问隔离通过直通设备、虚拟设备和排除设备列表控制访问边界；同步路径隔离则由 #2162 补齐，避免实时侧高优先级任务在 mutex 争用中被普通任务间接阻塞。

#2162 的 priority inheritance 不改变 VM 内存隔离边界，它解决的是实时任务内部的优先级反转。原有 `RawMutex` 只有 `owner_id` 和 wait queue，高优先级 waiter 阻塞时不会改变低优先级 owner 的调度地位；加入 PI 后，contended lock 会将 waiter 的 effective priority donation 给 owner，并在必要时触发 ready queue 重排，owner unlock 后再清理 donation。

| PI 状态或函数 | 作用 | 隔离意义 |
| --- | --- | --- |
| `base_sched_priority` | 保存调用方设置的基础优先级 | donation 不覆盖用户配置 |
| `donated_sched_priority` | 保存 mutex waiter 临时捐赠 | owner 可临时继承高优先级 |
| `effective_sched_priority` | 调度器实际观察的优先级 | RT FIFO 能按 donation 后优先级排序 |
| `mutex_wait_owner_id` | 记录当前任务等待的 owner | 支持 A 等 B、B 等 C 的链式传播 |
| `requeue_task_after_priority_change()` | 重新插入 ready queue | priority 改变后调度顺序立即生效 |

隔离能力的验证不只依赖源码说明，还应通过压力测试和异常注入形成证据。例如在智能侧执行 CPU/内存压力负载，同时持续测量 Axvisor RT 周期任务延迟和通信响应时间；在实时任务路径中构造低优先级 owner、高优先级 waiter 和中优先级干扰任务，验证中优先级任务不能长期阻止 owner 释放 mutex。

### 3.6 预期效果与边界

任务一预期交付一套可复现的 Axvisor AMP 实时运行方案，能够说明智能侧 guest 如何配置、实时 CPU 如何预留、Axvisor 关键路径如何测量，以及实时控制路径如何不被智能侧负载显著破坏。#2160 是实时 CPU 预留和隔离主线；#2163、#2164 和 #2165 补齐 Axvisor 启动、板级 I/O 和调频路径；#2161 和 #2162 作为实时任务调度与同步语义支撑。

当前边界需要明确记录：#2161 的 RT FIFO 只承诺单核调度语义，不承诺 SMP 全局实时调度；#2162 的 mutex PI 是 `sched-rt-fifo` 下的最小闭环，不等同于完整 POSIX `PTHREAD_PRIO_INHERIT`，也尚未实现 per-mutex waiter priority 重新计算、多锁 owner 的完整 donation 重算或 priority-aware wait queue。任务一的比赛价值在于形成可运行、可测量、可解释的实时性与隔离基础，而不是替代硬实时认证系统。

| 验收关注点 | 已有证据 | 后续补强方向 |
| --- | --- | --- |
| Axvisor CPU 隔离 | #2160 的实时 CPU 预留设计与 host glue 接入 | 增加板级多负载下的 RT 周期延迟记录 |
| Axvisor I/O 与调频路径 | #2163、#2164、#2165 的 guest 启动、SD 主机和 RK3588 governor 修复 | 增加板级 guest 加载、I/O 压力和频率 readout 记录 |
| RT FIFO 调度 | #2161 的 scheduler 单测和 ArceOS QEMU case | 作为预留 CPU 实时任务语义验证 |
| Mutex PI | #2162 的 QEMU PI 场景、clippy 和设计文档 | 补齐 per-mutex donation 重算和 priority-aware wait queue |
| 与任务二/三衔接 | Axvisor 实时侧支撑 GIPC/RT mailbox 和 AI 控制闭环 | 在端到端日志中加入 Axvisor RT 周期延迟指标 |

## 4. 任务二：客户机通信与协议设计

### 4.1 目标、范围与八项变更的依赖关系

任务二的目标是在同一 Axvisor 实例承载的 StarryOS/Linux 智能侧客户机与 ArceOS/RTOS 控制侧客户机之间，建立一条可启动、可寻址、可观测、可恢复的双向 IPv4/TCP 通信链路，并在链路之上提供控制指令、状态回传、心跳和错误通知的应用层语义。业务数据只经过标准网卡、Ethernet、IPv4 和 TCP；共享内存、HyperCall、裸 MMIO 和 vsock 不进入业务数据路径。

本任务不是孤立新增一个 socket 示例，而是由 8 项已提交变更逐层组成：

| 层次 | PR/提交 | 设计职责 | 对后续层的保证 |
| --- | --- | --- | --- |
| 启动与设备前置 | [#1926](https://github.com/rcore-os/tgoskits/pull/1926) | 保留 guest FDT 中 PSCI 信息 | 两个 guest 能按预期启动，vCPU/定时器基础路径不被破坏 |
| 虚拟设备前置 | [#1935](https://github.com/rcore-os/tgoskits/pull/1935) | 提供 VirtIO-MMIO 设备核心 | 客户机镜像和虚拟设备拥有稳定的 MMIO 接入基础 |
| 二层网络前置 | [#1927](https://github.com/rcore-os/tgoskits/pull/1927) | 双 guest VirtIO-net、MAC 和进程内 L2 switch | 两个客户机拥有隔离的网卡端点和可交换的二层帧路径 |
| IP 传输 | [#2155](https://github.com/rcore-os/tgoskits/pull/2155) | StarryOS/ArceOS QEMU VM、VirtIO-net 和拓扑配置 | 应用程序可以使用 `10.0.42.0/24` 私有子网进行 TCP 通信 |
| 应用协议 | [#2156](https://github.com/rcore-os/tgoskits/pull/2156) | GIPC 固定头帧、两端程序和编解码 | 控制、状态、错误和心跳具有稳定的线协议 |
| 可靠性 | [#2157](https://github.com/rcore-os/tgoskits/pull/2157) | TCP 分帧、超时、重连、序列窗口和恢复 | 字节流断连或重复请求不会被静默当作成功 |
| 验证 | [#2158](https://github.com/rcore-os/tgoskits/pull/2158) | QEMU 启动、rootfs 注入、日志和指标聚合 | 链路行为可由脚本复现并以非零退出码传递失败 |
| 启动补齐与观测 | [#2159](https://github.com/rcore-os/tgoskits/pull/2159) | StarryOS 网卡初始化、多请求和错误/恢复指标 | 文档地址成为实际启动配置，长运行结果可量化比较 |

八项变更的共同边界是任务二通信底座：它们不实现具体 AI 模型、不规定某种摄像头或 NPU 驱动，也不把控制动作绑定到某个板卡外设；任务三只需把推理结果编码到已有 payload，并使用本章定义的请求/响应流程。

#### 4.1.1 成功标准

任务二以可观察的端到端结果而不是单一模块编译成功作为完成标准：两个 guest 必须分别获得固定 MAC 和 IPv4 地址；StarryOS 必须能向 ArceOS TCP 4242 发送完整 GIPC CONTROL；ArceOS 必须完成 framing、CRC 和 sequence 校验并返回同序列 STATUS；断连后客户端必须按有限预算重新建连；运行结束必须给出请求数、成功数、应用错误、传输失败、重连、恢复、延迟和有效载荷吞吐。任一层失败都必须通过 `GIPC_*_ERROR`、`GIPC_STARRY_TIMEOUT` 或进程非零状态显式传播。

#### 4.1.2 设计选择与替代方案

| 方案 | 是否作为主通道 | 选择理由或排除原因 |
| --- | --- | --- |
| VirtIO-net + Axvisor 内部 L2 switch + TCP | 是 | 两端均经过标准 IP 协议栈；拓扑封闭、地址固定、无宿主网络依赖；TCP 适合控制指令和状态响应 |
| UDP/IP | 否，协议保留扩展能力 | 可降低连接开销，但必须在应用层实现 ACK、超时重传、乱序和重复包处理；当前需求优先采用 TCP 简化主路径 |
| TAP/bridge/NAT | 非默认变体 | 便于接入宿主或外部设备，但引入主机网络配置、路由、防火墙和环境差异，不适合作为默认可复现路径 |
| 物理网口 | 板级扩展 | 接近真实部署，但会引入网卡驱动、线缆、交换机和现场网络策略，不作为 QEMU 基线 |
| vsock | 仅辅助候选 | 不计入主要 IP 网络通道，不能替代网卡、路由和 TCP/IP 验收 |
| 共享内存/HyperCall/裸 MMIO | 禁止作为业务通道 | 不经过 IP 协议栈，无法满足赛题对网络拓扑、路由、端口和传输可靠性的要求 |

协议 crate 刻意保持 `no_std` 和传输无关：它只拥有线格式、CRC、序列窗口和纯状态机，不访问 socket、guest memory、MMIO 或 hypervisor 服务。StarryOS C 程序与 ArceOS Rust 程序分别承担 OS glue，使线协议可以被独立测试，也避免把 Linux/POSIX API 依赖引入可复用协议层。

![StarryOS 与 ArceOS 客户机 IP 通信架构](assets/network-communication.svg)

### 4.2 部署拓扑、地址规划与设备所有权

默认验收拓扑采用 Axvisor 进程内二层交换，不连接宿主桥、TAP、NAT 或物理上联。Axvisor 为每个 VM 创建一个 VirtIO-net MMIO 端点，并将端点注册到同一个内部交换机；交换机只在两个已注册端口之间转发 Ethernet 帧。每个端点的 MAC 地址在 VM TOML 中固定，避免 DHCP 或随机地址导致测试不可复现。

| 角色 | VM | 网卡 | MAC | IPv4/前缀 | 业务职责 |
| --- | --- | --- | --- | --- | --- |
| 智能侧 | StarryOS/Linux | `virtnet0` / `eth0` | `52:54:00:42:00:01` | `10.0.42.1/24` | 发起 CONTROL、HEARTBEAT，接收 STATUS/ERROR，汇总指标 |
| 控制侧 | ArceOS/RTOS | `virtnet0` / `eth0` | `52:54:00:42:00:02` | `10.0.42.2/24` | 监听 TCP 4242，校验请求，返回 STATUS/ERROR |
| 交换侧 | Axvisor | 内部 L2 switch | 不向 guest 暴露独立 IP | 二层转发 | 维护端口注册、帧转发和 guest 唤醒 |

StarryOS 启动时由 `/usr/bin/gipc-network-init.sh` 完成 `eth0` 配置：检查 `ip` 工具和接口存在，执行 `ip link set eth0 up`，配置 `10.0.42.1/24`，并尝试确保 `10.0.42.0/24` 直连路由存在。脚本在地址复核成功后打印 `GIPC_STARRY_NET_READY`，再由 autostart 启动客户端。当前脚本会容忍显式 `ip route add` 失败，因为内核通常在添加 /24 地址时自动生成直连路由；因此完整验收仍需保存 `ip route show dev eth0` 结果，不能只用 READY 标志替代路由证据。ArceOS 服务端发现 `eth0` 后调用 `ax_net::set_interface_ipv4(..., 10.0.42.2, 24)`，再绑定 `0.0.0.0:4242`。在当前仅有一个私网接口的 VM 中，实际可达面仍限定于该隔离子网，但代码本身不是“仅绑定 10.0.42.2”的 L3 白名单。

设备所有权保持在 Axvisor：VM 配置决定设备模型、MMIO 区域、IRQ 和 guest MAC；VirtIO-net 驱动负责队列和 DMA；交换机负责二层转发；guest 应用只拥有自己的 socket、协议会话和业务状态。块设备可以承载 rootfs 或镜像，但不承载 GIPC 业务数据。

#### 4.2.1 VM 资源与启动配置

| 配置项 | StarryOS VM | ArceOS VM |
| --- | --- | --- |
| VM ID | 1 | 2 |
| VM 名称 | `starry-virtio-net-peer` | `arceos-guest-ip-server` |
| vCPU | 1 个，绑定物理 CPU 1 | 1 个，绑定物理 CPU 2 |
| guest 内存 | `0x8000_0000` 起，大小 `0x4000_0000`（1 GiB） | `0x8000_0000` 起，大小 `0x2000_0000`（512 MiB） |
| 内核入口/加载地址 | `0x8020_0000` | `0x8020_0000` |
| DTB 加载地址 | `0x8000_0000` | `0x8000_0000` |
| 虚拟设备 | `virtnet0`, model=`virtio-net` | `virtnet0`, model=`virtio-net` |
| passthrough | 空 | 空 |

Axvisor host 使用 AArch64 `cortex-a72`、GICv3、4 vCPU 和 4 GiB QEMU 内存；board 配置同时列出两个 VM TOML。两个 guest vCPU 绑定不同物理 CPU，既避免配置冲突，也使通信延迟不会由同一 pCPU 上的串行调度人为造成。

#### 4.2.2 二层交换决策

内部 `VirtualSwitch` 维护 `SwitchPortId → port` 和 `MAC → SwitchPortId` 两个索引。端口注册会拒绝重复 ID 和重复 MAC；guest 发出的 Ethernet 源 MAC 必须与端口注册 MAC 一致，否则以 `SourceMacViolation` 丢弃并计数。已知单播只转发到目标端口；广播/组播复制给除源端口以外的活动端口，以支持 ARP；小于 14 字节的帧、已注销 generation 和未知上行单播分别计入独立 drop counter。

端口标识包含 VM ID、generation 和 device index，VM 重启后旧 generation 的端口不会冒充新实例；注册句柄和 active gate 负责让残留 `Arc` 安全失效。当前 Axvisor glue 虽能得到 switch 的 uplink 意图，但默认 GIPC 配置没有 host uplink worker、`-netdev`、TAP 或 bridge，因此主测试路径是完全位于 Axvisor 进程内的二层广播域。

### 4.3 网络启动时序与数据路径

启动时序必须先建立底层能力，再打开业务端口，避免客户端把“进程启动”误报为“网络可用”：

1. Axvisor 读取 board/QEMU/VM 配置，创建两个 VM、vCPU、地址空间、VirtIO-net MMIO 节点和内部交换机端口。
2. StarryOS guest 启动，完成 VirtIO-net 设备发现和 `eth0` 创建。
3. ArceOS guest 启动；应用进入 `main` 时先打印 `GIPC_RTOS_READY`，随后发现并配置 `eth0 = 10.0.42.2/24`，绑定成功后再打印 `GIPC_RTOS_LISTEN ip=10.0.42.2 port=4242`。前一个标志只表示应用入口已执行，后一个标志才证明网络和 listener 已就绪。
4. StarryOS profile 执行 autostart 和网络初始化脚本，打印 `GIPC_STARRY_NET_READY interface=eth0 address=10.0.42.1/24 peer=10.0.42.2`。两个 guest 并发启动，RTOS LISTEN 与 Starry NET READY 的相对先后不应作为协议正确性的前提。
5. StarryOS 客户端建立 TCP 连接，发送 CONTROL 或 HEARTBEAT；只有收到合法 STATUS/ERROR 后才记录一次应用层响应。
6. QEMU/测试运行器收集双方串口和客户端日志，`verify_metrics.py` 验证成功标志和正延迟/吞吐，`aggregate_metrics.py` 计算整体统计。

数据路径为：StarryOS socket → StarryOS TCP/IP → `eth0` VirtIO TX queue → Axvisor VirtIO-net backend → 内部 L2 switch → ArceOS VirtIO RX queue → ArceOS TCP/IP → TCP listener。响应沿相反方向返回。任何共享内存、HyperCall 或裸 MMIO 访问只属于设备实现或控制面，不属于业务 payload 路径。

#### 4.3.1 Host 构建与 rootfs 准备

`run-qemu-aarch64-starry-rtos-gipc.sh` 以 `ROOTFS_IMAGE` 为必需输入，并完成以下准备：

1. 从 `LLVM_OBJCOPY` 或 Rust sysroot 定位 `llvm-objcopy`；缺失时立即退出。
2. 使用 `cargo xtask arceos build -p arceos-guest-ip-server -c apps/arceos/build-aarch64-guest-ip-server.toml` 构建控制侧 ELF。
3. strip ELF 并转为可由 VM 配置装载的 raw binary。
4. 若未提供 `GIPC_STARRY_CLIENT_BIN`，使用宿主 C 编译器构建 `linux-client.c`。
5. 默认 `GIPC_INJECT_CLIENT=1`，通过 `debugfs` 向 rootfs 注入 `/usr/bin/gipc-starry-client`、`/usr/bin/gipc-network-init.sh` 和 `/etc/profile.d/99-gipc.sh`。设置 `GIPC_INJECT_CLIENT=0` 可使用预置镜像，避免重复修改 rootfs。
6. 调用 `cargo xtask axvisor qemu`，同时传入 board、QEMU 和 rootfs 配置。QEMU 总体运行门限为 180 秒。

#### 4.3.2 首包的 ARP、TCP 和 GIPC 路径

第一个 CONTROL 并不是直接从应用跳到对端 socket，而是依次经过以下协议和设备动作：

1. StarryOS 根据 `/24` 前缀判定 `10.0.42.2` 为直连邻居；邻居缓存为空时发送广播 ARP request。
2. Axvisor switch 将广播复制到除源以外的活动端口；ArceOS 回复单播 ARP response，switch 按固定目标 MAC 精确投递。
3. StarryOS 发起 TCP SYN，完成 SYN/SYN-ACK/ACK；客户端随后产生 40 字节 CONTROL（32 字节头 + 8 字节 payload）。
4. TCP/IP 栈把字节流封装为 IPv4/Ethernet 帧并提交 VirtIO TX descriptor chain。设备后端通过作用域 DMA 访问 descriptor，移除 `virtio_net_hdr` 后将 Ethernet frame 交给 switch。
5. switch 校验源 MAC、查找目标端口并将帧放入有界 ingress；目标端口通知 ArceOS vCPU。设备先把帧和 used ring 写回 guest，再触发 edge IRQ，保证 guest 观察中断时数据已可见。
6. ArceOS TCP 栈重组字节流；服务端先 `read_exact(32)`，校验固定头并获得 `payload_len`，再精确读取 payload 和验证 CRC。
7. 服务端生成同序列 STATUS，沿反向 VirtIO-net/IPv4/TCP 路径返回。客户端完成整帧校验后才计算成功 RTT。

最大 GIPC frame 为 `32 + 1200 = 1232` 字节；加上典型 20 字节 IPv4 头和 20 字节 TCP 头后为 1272 字节，低于常用 1500 字节 Ethernet MTU，因此最大应用帧在无额外 TCP option 的典型路径中不需要 IPv4 分片。当前 VirtIO-net profile 不依赖多队列、GSO、TSO 或 checksum offload。

#### 4.3.3 日志状态的精确定义

| 标志 | 精确含义 | 能否单独证明业务成功 |
| --- | --- | --- |
| `GIPC_RTOS_READY` | ArceOS 应用进入 `main` | 否，可能尚未配置 IP 或 bind |
| `GIPC_RTOS_LISTEN` | `eth0` 配置完成且 TCP bind 成功 | 否，只证明服务端 ready |
| `GIPC_RTOS_CONNECTED` | accept 到一个 TCP peer | 否，尚未证明 GIPC 帧合法 |
| `GIPC_RTOS_RECOVERABLE_ERROR` | 当前连接发生 EOF、解析或 I/O 错误，服务返回 accept | 否；故障注入中允许，正常基线应调查 |
| `GIPC_STARRY_NET_READY` | StarryOS 已发现接口并确认静态地址 | 否，未证明对端可达 |
| `GIPC_STARRY_STATUS` | 客户端收到同序列、magic/version/CRC 合法的 STATUS | 是，证明一次请求响应闭环 |
| `GIPC_STARRY_ERROR` | 收到 ERROR 或响应协议校验失败 | 否，作为失败证据 |
| `GIPC_STARRY_TIMEOUT` | 一个请求耗尽三次 attempt | 否，作为不可恢复失败证据 |
| `GIPC_STARRY_METRIC` | 一次 client process 的汇总 | 需结合字段判定 |

### 4.4 GIPC 应用层帧与消息语义

GIPC 使用固定 32 字节大端序头部加不超过 1200 字节 payload 的 framing，解决 TCP 字节流的粘包、拆包和边界恢复问题。头部布局如下：

![GIPC v1 帧格式与校验边界](assets/gipc-frame-format.svg)

| 偏移 | 长度 | 字段 | 语义与校验 |
| ---: | ---: | --- | --- |
| 0 | 4 | `magic` | 固定 `0x47495043`（`GIPC`），拒绝错误协议流 |
| 4 | 1 | `version` | 当前版本 `1`；未知版本由解码器 fail-closed 拒绝，错误码体系预留 `UnsupportedVersion` |
| 5 | 1 | `message_type` | `Hello=1`、`Control=2`、`Status=3`、`Error=4`、`Heartbeat=5`、`Ack=6` |
| 6 | 2 | `flags` | `ACK_REQUIRED` 等控制标志，按大端序编码 |
| 8 | 2 | `header_len` | 必须为 32，避免错误版本改变字段解释 |
| 10 | 2 | `payload_len` | 必须不超过 1200，读取完整帧前先做边界检查 |
| 12 | 4 | `sequence` | 请求/响应关联、重复检测和乱序判断 |
| 16 | 8 | `timestamp_ns` | 单调时钟时间戳，用于 RTT 和阶段耗时统计 |
| 24 | 2 | `error_code` | `None`、`UnsupportedVersion`、`InvalidLength`、`ChecksumMismatch`、`InvalidSequence`、`InvalidPayload`、`UnsupportedMessage`、`Busy` |
| 26 | 4 | `checksum` | CRC32；计算时将 checksum 字段置零 |
| 30 | 2 | 保留 | 当前置零，为后续兼容留出空间 |

消息处理规则如下：

- `HELLO`：服务端返回同序列、同 payload 的 STATUS，可用于会话建立或能力扩展；当前客户端主流程不主动发送。
- `CONTROL`：智能侧发起控制请求，当前 payload 精确为 8 字节 `00 00 00 01 00 00 00 00`。代码只规定长度必须为 8，尚未公开定义逐字段业务 schema；服务端把 payload 原样放入 STATUS，用于验证控制请求/状态响应链路，不应写成已经接入真实执行器。
- `STATUS`：控制侧返回原请求序列号和 payload；当前 `timestamp_ns=0`、`flags=0`、`error_code=None`。TCP profile 用合法 STATUS 作为应用交付确认，不发送独立 ACK。
- `ERROR`：只对可安全关联 sequence 的语义错误显式返回；当前包括 `InvalidSequence`、`InvalidPayload` 和 `UnsupportedMessage`。结构或 CRC 错误会终止当前连接并记录 recoverable error，而不是构造 ERROR。
- `HEARTBEAT`：服务端返回同序列、同 payload STATUS；具备线协议语义，但当前 C 客户端主流程只发送 CONTROL。
- `ACK`：协议保留类型；服务端当前接收后不响应，`IS_ACK` 和 `ReliableSession::acknowledge` 未接入 TCP 主路径。

#### 4.4.1 编码流水线

Rust `encode_frame` 不信任调用者提供的派生字段：它按常量重写 magic、version、header length，根据实际 payload 重写 payload length，并先把 checksum 清零。编码顺序为“规范化 header → 写入 32 字节头 → 复制 payload → 对完整帧计算 CRC32 → 回填 checksum”。CRC 使用反射式 CRC-32/IEEE：初值 `0xffff_ffff`，多项式 `0xedb8_8320`，结果按位取反；头部保留字节也在覆盖范围内。

C 客户端不直接序列化 C struct，避免 ABI padding 和对齐差异；它通过固定偏移和 `htons`/`htonl` 写网络序字段，64 位时间戳拆成两个 32 位网络序值。这使 C/Rust 两端不依赖相同编译器布局。

#### 4.4.2 分层解码流水线

服务端先固定读取 32 字节，再由 `decode_header` 验证 magic、version、message type、header length、payload bound 和 error code。只有固定头可信后才按 `payload_len` 读取载荷；`decode_frame` 再检查截断、尾随字节和 CRC。随后才执行 sequence 分类和消息分派。最大栈缓冲区固定为 `32 + 1200 = 1232` 字节，不根据不可信长度动态分配。

客户端响应校验集与 Rust decoder 不完全对称：它验证 payload 上限、magic、version、同 sequence 和 CRC，并只接受 STATUS 或显式处理 ERROR；当前未单独拒绝未知 flags、非零 reserved、错误的 response `header_len` 或未知 error code。因此技术边界应表述为“客户端当前 profile 校验集”，而不是宣称两端执行完全相同的全字段解析。

#### 4.4.3 本地解析错误与线上错误码

| 类别 | 典型成员 | 当前处理方式 |
| --- | --- | --- |
| `FrameError`：本地构造/解析失败 | `OutputTooSmall`、`TruncatedHeader`、`InvalidMagic`、`UnsupportedVersion`、`InvalidHeaderLength`、`PayloadTooLarge`、`UnknownMessageType`、`UnknownErrorCode`、`ChecksumMismatch` | 服务端结束当前连接，外层记录 `GIPC_RTOS_RECOVERABLE_ERROR` 并继续 accept |
| `ErrorCode`：可信帧内可传递错误 | `InvalidSequence=4`、`InvalidPayload=5`、`UnsupportedMessage=6` | 返回同 sequence、空 payload、带 CRC 的 ERROR；客户端记录 code/seq 并计入应用错误 |
| 预留错误码 | `UnsupportedVersion=1`、`InvalidLength=2`、`ChecksumMismatch=3`、`Busy=7` | 线格式编号稳定，但当前结构/完整性解析路径不会发送这些 ERROR |

当 header 或 sequence 尚未可信时，服务端不尝试回复可能错误关联的 ERROR，而采用 fail-closed 断连。这一区分防止文档把“错误码枚举存在”误写成“所有解码错误都已在线返回”。

### 4.5 TCP 可靠性、状态机与异常恢复

TCP 只保证有序字节流，不保证应用请求已经被处理，因此实现仍需维护应用层状态：

![GIPC 请求响应与异常恢复时序](assets/gipc-request-sequence.svg)

| 状态/事件 | StarryOS 客户端行为 | ArceOS 服务端行为 |
| --- | --- | --- |
| 建连 | 最多尝试 3 次，socket 读写超时为 1 秒 | `accept` 新连接并建立会话 |
| 发送 | 为每个进程内请求分配递增 `sequence`，完整写入 32-byte header + payload | `read_header` 后按 `payload_len` 精确读取完整帧 |
| 正常响应 | 校验 magic/version/type/sequence/CRC，记录 STATUS 和 RTT | 返回同序列、同 payload STATUS |
| ERROR | 读取 `error_code`，计入 `errors`，拒绝伪造成功 | 对非法 payload、序列或消息类型返回 ERROR |
| 读写超时/断连 | 关闭当前 socket，增加 timeout，重新 connect 并重发未完成请求 | 记录 recoverable error，关闭当前会话并继续 accept |
| 重复 sequence | 客户端只接受当前请求的匹配响应 | 同一连接内分类 `Duplicate`；CONTROL 返回同序列、同 payload STATUS，不进入 `New` 分派 |
| 旧/乱序 sequence | 计入协议错误 | 分类 `OutOfOrder`，返回 `InvalidSequence` |
| 重试预算耗尽 | 输出 `GIPC_STARRY_TIMEOUT` 并返回非零 | 由上层日志记录会话失败，不静默降级到非 IP 通道 |

每个请求的最大总尝试次数为 3（首次 + 最多 2 次后续尝试）；重连不是成功本身，只有收到匹配的 STATUS 才计入 `success`。同一进程的序列号从 1 递增，但客户端当前每个请求成功或失败后都会关闭 socket，下一个请求建立新连接。由于 ArceOS 服务端也按 TCP 连接创建 `ReliableSession`，sequence 窗口只在单连接内有效，不能宣称跨连接 exactly-once。非幂等控制动作必须在未来业务层增加持久 request ID、执行结果缓存或动作幂等约束。

#### 4.5.1 客户端状态机

客户端参数 `<peer-ip> [request-count]` 中请求数范围为 1..1000。每个请求执行 `PREPARE → CONNECT → SEND → READ_HEADER → READ_PAYLOAD → VALIDATE`：

- `write_full` 循环处理短写；`read_full` 循环处理短读，EOF 映射为 `ECONNRESET`。
- 每次 attempt 新建 socket，并设置 `SO_SNDTIMEO`/`SO_RCVTIMEO` 为 1 秒。
- connect、send、header read、payload read 或 payload 超限进入 `CLOSE → RETRY`；三次耗尽为 `TIMED_OUT`。
- magic、version、sequence 或 CRC 错误、ERROR 帧和非 STATUS 类型属于协议/应用错误，立即结束该请求，不重试。这与 README 中“protocol failure 也重试”的宽泛描述不同，技术方案以实际代码为准。
- 成功尝试从编码前的 `CLOCK_MONOTONIC` 时间开始计时，到完整 STATUS 校验结束；先前失败 attempt 的等待时间不计入成功 RTT。

#### 4.5.2 服务端状态机

ArceOS 使用单线程 `accept` 循环。每个连接新建 session，循环读取完整帧、执行 sequence 分类和消息分派。EOF、解码或写入失败使 `serve_connection` 返回，外层打印 recoverable error 后继续 accept，因此单个连接异常不会终止整个服务。

`RetryPolicy(1000 ms, 3)` 虽被放入 server session，但 TCP 服务端当前只调用 `observe`，不调用 `begin`、`acknowledge` 或 `poll_retry`；它不构成服务端 I/O deadline。`ReliableSession` 库中的 `max_retries` 表示首次发送后的重传次数，而 C 客户端的 3 是总 attempt 数，两者是独立状态机，不能混为一谈。

#### 4.5.3 序列窗口与回绕

| 输入 sequence | 分类 | 状态变化 |
| --- | --- | --- |
| 连接内首个值 | `New` | 记录为 `last_received`，不要求必须从 1 开始 |
| 等于 `last_received` | `Duplicate` | 不推进窗口 |
| `sequence.wrapping_sub(last) < 2^31` 且不相等 | `New` | 推进窗口，允许 u32 自然回绕和半序空间内向前跳号 |
| 其他值 | `OutOfOrder` | 不推进窗口，返回 `InvalidSequence` |

该算法不是严格的 `last + 1` 连续窗口；它允许向前跳号，适合请求关联和陈旧包识别，但不检测“缺失的中间序列”。

#### 4.5.4 故障与恢复矩阵

| 故障 | 检测点 | 恢复动作 | 验收证据 |
| --- | --- | --- | --- |
| 首次连接被对端关闭 | client header read EOF | 关闭 socket，下一 attempt 新建连接并重发同 sequence | 确定性测试断言 `attempts=2 timeouts=1 reconnects=1 recovery=1` |
| TCP 短读/短写 | `read_full`/`write_full` | 循环补齐，未完成前不解析帧 | 最终 STATUS 或 I/O 失败 |
| 服务端连接 EOF/坏帧 | `read_exact`/decoder | 结束当前连接，外层继续 accept | `GIPC_RTOS_RECOVERABLE_ERROR`，随后可再次 CONNECTED |
| CONTROL 长度不是 8 | server dispatch | ERROR `InvalidPayload`，保持连接 | 客户端 `GIPC_STARRY_ERROR code=5` |
| 旧 sequence | server observe | ERROR `InvalidSequence`，保持连接 | error code 4 |
| response CRC/type/sequence 错误 | client validate | `errors++`，该请求失败且不重试 | `GIPC_STARRY_ERROR code=protocol` |
| 三次传输 attempt 均失败 | client retry budget | 输出 TIMEOUT，总进程最终非零 | `GIPC_STARRY_TIMEOUT seq=... attempts=3` |

服务端 accepted stream 当前没有独立 receive/send/idle deadline，且逐连接串行服务；对端连接后长期不发送完整头可能占用服务循环。这是当前 profile 的可用性边界，不应写成已实现“服务端半连接超时回收”。

### 4.6 安全边界、故障模型与访问控制

默认拓扑通过“不连接宿主网络”缩小攻击面：没有默认网关、NAT、宿主 bridge 或 uplink worker。ArceOS 实际 bind `0.0.0.0:4242`，在当前只有私网接口的 VM 中实际暴露面仍是封闭二层域，但代码没有 guest firewall、peer IP allowlist 或只绑定 `10.0.42.2` 的 L3 控制。配置固定对端 MAC/IP，运行日志打印接口、地址、peer 和端口；后续板级或桥接变体必须单独记录二层边界、路由、NAT 和防火墙规则，不能沿用默认隔离结论。

协议解码在执行控制动作前完成 magic、version、header_len、payload_len、message_type、error_code、sequence 和 CRC 校验；超过最大 payload、未知类型、非法错误码或校验失败均不得进入控制状态机。错误路径必须产生 ERROR 或明确的断连/错误日志，验证脚本以非零返回传播失败。

故障模型覆盖：接口不存在、地址配置失败、服务端尚未监听、TCP 建连失败、半帧/粘包、CRC 损坏、版本不兼容、非法 payload、重复请求、乱序请求、读写超时和服务端重启。共享内存、HyperCall、裸 MMIO 和 vsock 不得作为这些故障的隐式 fallback。

#### 4.6.1 信任边界与已实现控制

| 边界 | 已实现机制 | 安全/健壮性作用 |
| --- | --- | --- |
| guest 应用 | 只使用 POSIX/ax_std TCP socket；协议 crate 不访问 socket、MMIO 或 guest memory | 防止应用绕过 IP 主通道 |
| VM 设备 | 两个 VM `passthrough=[]`，各自只获得独立 `virtnet0`、MMIO/IRQ 和作用域 DMA grant | 限制设备和内存访问范围 |
| switch 注册 | port ID 与 MAC 均要求唯一，RAII 注销，generation active gate | 阻止重复配置和旧 VM 端口残留 |
| Ethernet ingress | 源 MAC 必须等于端口注册 MAC；小帧、inactive generation 丢弃 | 二层 anti-spoof 和畸形帧隔离 |
| 转发 | 已知单播只给目标；未知单播不向本地端口泛洪；广播/组播只给其他 active port | 减少横向暴露，同时保留 ARP |
| 协议输入 | 固定 1232 B 缓冲上限、结构校验、CRC、sequence 分类、CONTROL 长度 8 | 不按不可信长度无界分配，错误不进入控制分派 |

switch 的 `source_mac_violation`、`undersize_drop`、`inactive_generation_drop`、`duplicate_mac_rejected` 和 `unknown_unicast_drop` 使用 Relaxed 原子统计；这些计数只用于观测，不参与同步或改变转发决策。anti-spoof 只校验 Ethernet 源 MAC，不验证 IPv4 源地址。

#### 4.6.2 错误处置分级

| 错误级别 | 示例 | 响应策略 |
| --- | --- | --- |
| 可关联语义错误 | 旧 sequence、CONTROL 长度错误、把 STATUS/ERROR 作为请求 | 返回同 sequence ERROR，分别使用 `InvalidSequence`、`InvalidPayload`、`UnsupportedMessage` |
| 结构/完整性错误 | header 不足、magic/version/type/header length/error code 非法、payload 截断、CRC mismatch | 不信任 sequence，关闭当前连接；服务端记录 recoverable error 并继续 accept |
| 网络启动错误 | Starry 缺少 `ip`、`eth0` 或地址复核失败；ArceOS 缺 `eth0`、地址配置或 bind 失败 | 输出 NET_ERROR/RTOS_ERROR，阻止业务流程继续 |
| 客户端传输错误 | connect/send/read/EOF/payload 读取失败 | 计入“超时或传输尝试失败”，按 3 次总预算重试；耗尽后 TIMEOUT |
| 业务响应错误 | ERROR、非 STATUS、响应 magic/version/sequence/CRC 错误 | 计入 application error，当前请求立即失败 |

#### 4.6.3 非安全属性与上线边界

当前方案没有 TLS、消息签名、身份认证、密钥协商、L3/L4 ACL、连接速率限制或服务端 idle timeout。CRC32 只检测偶发损坏，不提供机密性、来源认证或抗恶意篡改。CONTROL 当前只校验 8 字节长度，没有逐字段授权规则。默认威胁模型是“Axvisor 正确隔离、只有两个受控 VM 端口、无外部 uplink”。

如果未来启用 TAP/bridge/NAT/物理网口，必须把下列策略作为新部署的显式前置条件：默认拒绝；仅允许源 `10.0.42.1` 到目的 TCP 4242；限制 bridge/TAP 和 guest 对宿主管理面的访问；在 bridge family 防止外部伪造两个 guest MAC；记录 stateful response 规则和 drop counter；跨不可信网络时增加认证加密。多租户扩展还需使用独立 switch/VLAN/ACL，因为广播和组播会复制到所有 active port。

可用性方面，当前单线程服务端可能被“连接后不发送完整 32 字节头”的 peer 长期占用；客户端 1 秒 socket deadline 也不等同于显式非阻塞 connect deadline。这些限制应在真实外联或多租户部署前通过服务端 read/write/idle deadline、并发连接上限和 rate limit 补齐。

### 4.7 可观测性、指标定义与验收证据

客户端输出 `GIPC_STARRY_STATUS` 和 `GIPC_STARRY_METRIC`，聚合器输出 `GIPC_AGGREGATE`。指标定义如下：

![GIPC 验证与指标流水线](assets/gipc-observability.svg)

| 指标 | 计算方式 | 用途 |
| --- | --- | --- |
| `requests` / `success` | 请求总数 / 收到合法 STATUS 的请求数 | 基础计数 |
| client `success_rate` | `1[success == requests]`，值为 0 或 1 | 进程级“是否全部成功”布尔标志，不是小数比例 |
| aggregate `success_rate` | `Σsuccess / Σrequests`，输出 6 位小数 | 多样本的真实请求成功比例 |
| 应用层错误 | ERROR 帧、CRC/版本/类型/序列校验失败计数 | 区分业务拒绝和协议异常 |
| `timeouts` | connect、write、header read、payload 超限/读取失败的 attempt 次数 | 实际口径是“超时或传输尝试失败”，并非每次都经历 deadline |
| `attempts` | 每次创建 socket 并尝试 connect 均累加 | 衡量请求的传输成本 |
| `reconnects` | 同一逻辑请求中 attempt>0 且 connect 成功的次数 | 正常请求之间主动重新连接不计入 |
| client `recovery` | `1[success>0 ∧ reconnects>0]` | 进程级恢复布尔标志，不是恢复率 |
| aggregate `recoveries` | 日志中 `recovery=1` 的 sample 数 | 成功恢复运行次数，不是恢复请求百分比 |
| `rtt_ns` | 成功 attempt 从发送前到完整 STATUS 校验后的平均值 | 不包含此前失败 attempt 的耗时 |
| `rtt_p50_ns/p95_ns` | 对每条 metric 的 run 级平均 RTT 排序后取 `floor((M-1)q)` | 当前是 run 平均值的经验分位点，不是每请求原始分位点 |
| `throughput_bps` | 对每个成功响应计算 `payload_len×10^9/RTT` 后求平均 | 字段名沿用 bps，但代码未乘 8，实际量纲是有效响应 payload B/s |

聚合器对各 run 的 `errors`、`timeouts`、`reconnects` 和 `recovery` 求和；只要总成功数等于总请求数且应用错误为零，即使存在已经恢复的 timeout/reconnect，仍返回成功。这使故障注入结果能够保留恢复事件，而不是把“发生过故障”和“最终未恢复”混为一谈。

#### 4.7.1 自动门禁的实际判据

| 层次 | 工具/配置 | 当前判据 |
| --- | --- | --- |
| QEMU 在线门禁 | `qemu-aarch64-starry-rtos-gipc.toml` | 180 秒；success regex 要求 `RTOS_READY → STARRY_STATUS → STARRY_METRIC`；fail regex 捕获 panic、RTOS_ERROR、STARRY_TIMEOUT |
| 单日志验证 | `verify_metrics.py` | 必须有 STATUS/METRIC；不得有 STARRY_TIMEOUT、RTOS_ERROR、STARRY_ERROR；首个 metric 必须 `requests==success` 且 RTT/吞吐>0 |
| 多日志聚合 | `aggregate_metrics.py` | 必须找到 metric；输出成功率、错误、超时、重连、恢复、P50/P95 和平均吞吐；全部请求成功且 errors=0 才返回 0 |

`GIPC_AGGREGATE` 是保存 guest log 后由 host 脚本生成的离线产物，不是 guest 原生日志，也不在当前 QEMU success regex 中。在线 regex 尚未直接要求 `GIPC_STARRY_NET_READY`、`GIPC_RTOS_LISTEN`、`GIPC_STARRY_ERROR` 或 `GIPC_RTOS_RECOVERABLE_ERROR`；正式交付采用比自动 regex 更强的检查清单：地址和 listen marker 必须存在，所有 sequence 对应，`requests=success`、`errors=0`、RTT/吞吐为正，正常基线不出现 recoverable error；故障注入若出现 recoverable error，随后必须重新 CONNECTED、收到 STATUS 且 `recovery=1`。

#### 4.7.2 分层验证矩阵

| 验证层 | 命令或用例 | 主要覆盖 | 不替代的证据 |
| --- | --- | --- | --- |
| 协议单元 | `cargo test -p guest-ip-protocol` | round-trip、CRC 损坏、尾随字节、retry budget、duplicate/out-of-order | 不启动 socket、VirtIO-net 或 guest |
| 静态质量 | protocol/server check+Clippy、C `-Wall -Wextra -Werror`、Python compile、fmt/diff-check | 两端构建、类型和脚本语法 | 不证明实际包路径 |
| 确定性恢复 | `test_linux_client.py` | mock peer 第一次 accept 后立即关闭，第二次返回合法 STATUS；断言 attempts=2/timeouts=1 | 只证明 host C client 恢复，不替代双 guest 性能 |
| 双 guest 端到端 | `run-qemu-aarch64-starry-rtos-gipc.sh` | Axvisor、两个 VM、VirtIO-net、ARP/TCP、GIPC 请求响应和日志链 | 性能数值应从归档日志读取，不编造固定 P50/P95 |
| 离线强验收 | `verify_metrics.py guest.log` + `aggregate_metrics.py guest.log` | marker、错误、成功率、RTT 和吞吐统计 | 依赖输入日志采样范围 |

建议归档以下结构化日志模板，尖括号字段由实际运行填充：

```text
GIPC_RTOS_READY
GIPC_RTOS_LISTEN ip=10.0.42.2 port=4242
GIPC_STARRY_NET_READY interface=eth0 address=10.0.42.1/24 peer=10.0.42.2
GIPC_RTOS_CONNECTED peer=10.0.42.1:<ephemeral-port>
GIPC_STARRY_STATUS seq=1 payload=8 attempts=<A> timeouts=<T>
GIPC_STARRY_METRIC requests=<N> success=<S> success_rate=<0|1> errors=<E> timeouts=<T> attempts=<A> reconnects=<R> recovery=<0|1> rtt_ns=<mean> throughput_bps=<mean-effective-B/s>
GIPC_METRICS_OK
GIPC_AGGREGATE requests=<N> success=<S> success_rate=<ratio> app_errors=<E> timeouts=<T> reconnects=<R> recoveries=<runs> rtt_p50_ns=<P50> rtt_p95_ns=<P95> throughput_avg_bps=<B/s>
```

## 5. 任务三：AI 联动控制应用设计

![任务三 AI 语音识别与实时控制闭环架构](assets/task3-ai-control.svg)

### 5.1 任务目标

任务三目标是在任务一和任务二基础上构建完整应用闭环，证明 StarryOS 智能侧完成 RK3588 语音识别后，能够把识别结果转换为受限控制指令，并通过 Axvisor 实时侧任务驱动双轮足机器人动作。该任务不是单纯跑通一个模型，而是把模型应用生态、推理性能、应用启动和实时控制闭环放在同一条链路中验证。

任务三的工作内容分为三条主线：

| 工作方向 | 目标 | 关键内容 | 输出证据 |
| --- | --- | --- | --- |
| 模型应用生态适配 | 让 StarryOS 智能侧具备运行 RK3588 语音识别模型的应用环境 | SenseVoice/RKNN runtime 接入，fbank、LFR、CMVN、CTC 解码链路对齐，样例 wav 输入和命令词映射 | `control_voice.wav`、推理日志、转写结果 |
| 模型性能优化 | 缩小 StarryOS guest 与原生 Linux 的推理和 NPU 提交差距 | guest vCPU 绑定 A76 大核、板级日志降噪、card1 ioctl 聚合计时、readahead 窗口扩大、RK3588 governor 归因修复 | `sensevoice-perf.svg`、串口 `[perf]` 日志、`test-plan.md` 性能表 |
| 应用启动优化 | 缩短从 Axvisor 启动、guest 加载到语音应用可执行的等待时间 | SD/rootfs/模型加载路径梳理，guest autostart，样例输入放置，模型冷读瓶颈定位 | `minicom_output.jpg`、启动串口日志、模型加载计时 |

本项目的实物演示场景为双轮足机器人：右侧 StarryOS 智能侧适配 RK3588 的语音识别模型应用生态，识别成功后将中文语音转换为 `forward`、`back`、`left`、`right`、`stop` 等有限指令；指令再进入 Axvisor 预留 CPU 上的实时任务，由实时控制闭环执行轮足平衡与电机控制。交付目录中的三份素材用于支撑该场景的展示证据：

| 素材 | 文件 | 证明内容 |
| --- | --- | --- |
| 语音输入 | [control_voice.wav](assets/control_voice.wav) | 智能侧语音识别输入样例，用于触发前进、后退、转向或停止等控制语义 |
| 演示视频 | [video.mp4](assets/video.mp4) | 系统部署到双轮足机器人后的端到端动作展示 |
| 启动串口截图 | [minicom_output.jpg](assets/minicom_output.jpg) | 开发板启动 Axvisor/客户机/实时任务时的串口输出证据 |

### 5.2 任务三技术架构

任务三采用“StarryOS 智能侧 + Axvisor 实时侧”的 AMP 应用架构。StarryOS 侧负责模型应用生态和语音识别，Axvisor 侧负责接收受控命令并在预留实时 CPU 上执行 8ms 控制任务。两侧之间不传递任意脚本或不受限控制量，而是传递有限命令 token，降低智能侧误识别、卡死或应用异常对实时侧的影响。

```text
control_voice.wav
  -> StarryOS SenseVoice RKNN 推理
  -> 中文短语识别与命令词映射
  -> @@RT command console marker
  -> Axvisor guest console observer
  -> RT mailbox command
  -> 8ms wheel balance loop
  -> IMU + motors
  -> 双轮足机器人动作
```

这条架构把任务三的工作边界拆清楚：StarryOS 负责“模型能跑、跑得快、应用能启动”；Axvisor 实时侧负责“命令能被实时任务接收、控制周期稳定、动作可验证”。后续如果将 console 原型替换为任务二的结构化 GIPC CONTROL 消息，模型生态、性能优化和实时控制闭环都可以保持不变。

### 5.3 模型应用生态适配

模型应用生态适配的核心是让 StarryOS guest 具备承载 RK3588 语音识别应用的必要运行环境，而不是只在宿主 Linux 上证明模型可用。本项目在开发板阶段选型 **SenseVoice 语音识别**作为智能侧模型，落地链路为：

```text
Axvisor (EL2, SD 卡加载 guest)
  -> StarryOS guest (passthrough, vCPU 绑定 A76 大核)
     -> /dev/dri/card1 (rknpu DRM 重实现)
        -> librknnrt (C API) -> RK3588 NPU (fp16-scaled 模型)
           -> CPU 侧 CTC 解码
```

适配工作包括输入音频格式、特征前处理、NPU runtime 调用和后处理命令映射四个层面。`sensevoice_rknn_npu.py` 以 16 kHz 单声道 wav 为输入，完成 fbank80、LFR、CMVN 等前处理后调用 RKNN runtime，在 RK3588 NPU 上执行 SenseVoice encoder，并将中文短语映射到有限控制命令集合。固定命令集合包括前进、后退、左转、右转和停止，避免智能侧直接注入任意速度、偏航角速度或电机电流。

正确性方法是把推理路径与社区上游运行时（happyme531/SenseVoiceSmall-RKNN2 及模型作者的 rkvoice-stream）逐项对齐：tensor 查询枚举、输入构造（4 个提示帧 + LFR 语音帧）、kaldi 兼容 fbank 前端、CMVN 符号、输出布局与 CTC 解码；前端在宿主机用 kaldi-native-fbank 数值对拍（fbank 偏差 ≤ 3e-4，LFR+CMVN 后 ≤ 4e-5）。板上 zh/en 参考 wav 转写通过（fp16 精度边缘，漏 1-2 字），推理语义与原生 Linux 一致。

关联 PR/提交如下：

| PR/提交 | 对应工作 | 说明 |
| --- | --- | --- |
| [`672793b95`](https://github.com/rcore-os/tgoskits/commit/672793b9572855a3bd7b795c1151c8490aad4542) | StarryOS QEMU SenseVoice 应用 | 增加 CPU 版 SenseVoice ASR 应用、模型资产、glibc runtime 和 zh/en 样例测试，为模型生态适配提供可复现基线 |
| [`a1444c0ee`](https://github.com/rcore-os/tgoskits/commit/a1444c0ee68496d1db27e35a1557277667ba711c) | Axvisor + StarryOS E2E 用例 | 将 StarryOS SenseVoice 应用放入 Axvisor QEMU guest 中端到端运行，验证 hypervisor 层不破坏应用路径 |
| [`7d292062c`](https://github.com/rcore-os/tgoskits/commit/7d292062c9a17c6318c3f294e59252a1f217f81d) | RK3588 NPU 板级应用骨架 | 增加 `sensevoice-rknn` 板级应用、RKNN runtime 调用、fbank/LFR/CMVN/CTC 和 host frontend 数值检查 |
| [`54ad820b3`](https://github.com/rcore-os/tgoskits/commit/54ad820b3f5484d8f4f46586c6136f9c9c5ed06c) | Axvisor + StarryOS + NPU 板级链路 | 打通 OrangePi 5 Plus 上 Axvisor、StarryOS guest、RK3588 NPU passthrough 和 librknnrt 的实际运行路径 |

### 5.4 模型性能优化

智能侧推理最初与原生 Linux 差距明显（单条推理 2.96s vs 1.04s，模型加载 41.2s vs 0.65s）。通过在 card1 ioctl 层增加聚合计时仪表，逐项定位并收敛：

| 优化项 | 问题 | 实测效果（板级，标注日志档与调频状态） |
| --- | --- | --- |
| guest vCPU 绑定大核 | guest 运行在 A55 小核（实发约 1175 MHz） | Info 档：模型加载 41.2s→27.1s，推理 2.96s→2.60s |
| 板级日志降噪 | 每 ioctl 的 info 行 + submit 结构体 dump 约 100KB/轮串口流量 | Error 档：推理 2.60s→1.72s，rknn_init 1.22s→0.66s，加载 26.30s（冷读 25.41s，19.2 MB/s） |
| 调频 governor 拓扑归因修复 | SMP=1 guest 的 busy 恒记到 A55 簇，实际运行的大核被降到 408 MHz | Error 档＋动态调频：推理 1.72s→1.46s，rknn_init 0.66s→0.50s；冷读 29.82s（16.4 MB/s，突发 I/O 间隙降档所致） |
| readahead 窗口 1 MiB | 32 页窗口下 490 MB 模型读发起约 3800 个请求，间隙损失 22% 总线带宽 | 请求与 IDMAC 链上限对齐（板测进行中） |

收敛后 NPU 提交路径达原生水平（7.78 ms/次 vs 原生约 7.5 ms）；剩余模型加载差距由 SD HighSpeed 总线上限决定（冷读实测 16.4～19.2 MB/s，为 24.75 MB/s 总线极限的 66%～78%），后续方向为 UHS-I（SDR104/DDR50）使能，前置的 1.8 V 电压轨与协议状态机工作已在内部分支完成。

优化前后与原生 Linux 的对比图如下，三配置串口原始数据见 `test-plan.md` §5.3：

![SenseVoice 推理与模型加载性能对比](assets/sensevoice-perf.svg)

关联 PR/提交如下：

| PR/提交 | 对应工作 | 说明 |
| --- | --- | --- |
| [#2166](https://github.com/rcore-os/tgoskits/pull/2166) | RK3588 guest 性能收敛 | 覆盖 guest A76 绑核、日志降噪、card1 ioctl 聚合计时、readahead 扩大等性能优化，形成 `sensevoice-perf.svg` 和 `test-plan.md` 中的实测数据 |
| [#2165](https://github.com/rcore-os/tgoskits/pull/2165) | RK3588 governor 拓扑归因修复 | 修复 SMP=1 guest busy 归因到错误 CPU 簇的问题，使实际运行大核不再被错误降频，是动态调频组推理 1.46s 的前置优化 |
| [`54ad820b3`](https://github.com/rcore-os/tgoskits/commit/54ad820b3f5484d8f4f46586c6136f9c9c5ed06c) | 板级 NPU 执行路径 | 在 OrangePi 5 Plus 上跑到 `rknn_init/run/outputs`，为后续性能计时和差距定位提供实际板级路径 |

### 5.5 应用启动优化

应用启动优化关注从 Axvisor 上电启动到 StarryOS 语音识别应用可执行的整段路径。任务三不是只看推理函数耗时，还要看开发板上是否能稳定加载 guest、挂载 rootfs、找到模型文件、初始化 RKNN runtime，并在演示输入到达前完成准备。

当前启动路径中，Axvisor 从 SD 卡加载 StarryOS guest，guest 内部启动语音识别应用并读取模型文件。模型加载时间受 rootfs、SD 冷读、文件缓存和 runtime 初始化共同影响，因此文档中把模型加载、`rknn_init` 和单条推理分开记录。启动串口截图 [minicom_output.jpg](assets/minicom_output.jpg) 用于证明系统已经进入板级运行环境，演示视频 [video.mp4](assets/video.mp4) 用于证明语音命令能够驱动机器人动作。

关联 PR/提交如下：

| PR/提交 | 对应工作 | 说明 |
| --- | --- | --- |
| [`58cb3b197`](https://github.com/rcore-os/tgoskits/commit/58cb3b197eaf3f9eb00977939fbb05d39ad35acf) | Axvisor guest 自动加载与失败快返 | 将 StarryOS guest kernel 改为构建期嵌入，减少手工 rootfs 注入步骤，并在 guest 卡死时快速失败 |
| [`2b92958ff`](https://github.com/rcore-os/tgoskits/commit/2b92958ff8254381eca63a6ab692ef18ed58cd18) | rootfs overlay 注入修复 | 修复 debugfs 绝对路径注入导致文件不可见的问题，使 SenseVoice rootfs 资产可被 guest 稳定解析 |
| [`e325b5aa2`](https://github.com/rcore-os/tgoskits/commit/e325b5aa2e7234088135b85cc039b572c88883f5) / [`b0022043f`](https://github.com/rcore-os/tgoskits/commit/b0022043f5ec18e3de6c8616c42d1c31e27ef19a) | 资产下载稳定性 | 为 SenseVoice 模型、runtime 和样例音频下载增加镜像 fallback、断点续传和重试，降低复现环境网络波动对应用启动的影响 |

### 5.6 双轮足机器人实物闭环

双轮足机器人同时具备倒立摆平衡、差速转向和腿部高度调节特征。与只控制轮式小车不同，轮足平台需要持续估计机体倾角、角速度、前向速度和偏航角速度，并在一个固定周期内完成传感器读取、状态估计、控制律计算和电机输出。如果控制周期抖动过大，机器人会表现为前后摆动、转向迟滞，严重时会失稳倒地。

`rt-robot` 分支中已经形成过该实物链路的原型实现，关键路径包括：

| 环节 | 原型代码锚点 | 作用 |
| --- | --- | --- |
| 语音识别 | `sensevoice_rknn_npu.py` | 在 StarryOS 侧通过 RK3588 NPU 运行 SenseVoice，识别中文语音命令 |
| 命令标记 | `@@RT <token>` console 行 | 将识别结果转换为 `forward`、`back`、`left`、`right`、`stop` 等固定 token |
| Host 侧监听 | `os/axvisor/src/wheel/console.rs` | 监听 guest console 输出，解析 `@@RT` 命令，不改变原始串口流 |
| RT mailbox | `os/axvisor/src/wheel/command.rs` | 把命令编码为单字节消息，发送到实时控制侧 |
| 控制执行 | `os/axvisor/src/wheel/hardware.rs` | 在 RT 任务中读取 IMU/电机状态，计算扭矩并写入 UART 电机协议 |
| 控制算法 | `os/axvisor/src/wheel/controller.rs`、`control.rs`、`ekf.rs`、`model.rs` | 组织 EKF 状态估计、动力学预测、LQR 扭矩控制和舵机腿部几何 |

实物链路使用 console 标记作为概念验证通道：StarryOS 内的 Python 程序在识别到语音后输出 `@@RT forward` 等行，Axvisor 在 guest console mux 处观察输出并转发给实时侧。该方式避免在演示阶段额外引入 virtio 控制通道，便于快速验证“AI 推理结果进入实时控制闭环”。正式工程化时，仍可复用任务二的 GIPC/IP 协议，把 `@@RT` 命令替换为结构化 CONTROL 消息。

### 5.7 AI 模型选型原则

模型选型遵循轻量、可部署、可复现和结果可解释原则。对于视觉感知类场景，可选择 YOLO 系列轻量模型或已有板级 NPU 示例进行验证；对于非视觉场景，也可使用分类、检测或规则增强模型。模型不追求复杂度最大，而是强调推理结果能够稳定转换为控制语义。

双轮足机器人演示采用语音识别作为 AI 输入，原因是语音命令可以直接映射为有限动作集合，便于演示“AI 识别结果进入实时控制闭环”，同时不会把不稳定的连续控制量交给智能侧模型生成。

### 5.8 模型部署方式

智能侧客户机负责模型文件、推理运行时和输入数据管理。QEMU 阶段可以使用离线样例输入验证链路；开发板阶段可以结合实际摄像头、NPU 或预置输入源进行演示。仓库中 `drivers/npu/` 和 `test-suit/starryos/normal/board-orangepi-5-plus/npu-yolov8/` 可作为 NPU/YOLO 板级验证材料的组织参考。

在机器人实物演示中，智能侧部署在 Orange Pi 5 Plus 上，使用 RK3588 NPU 运行语音模型；控制侧实时路径位于 Axvisor/RT 任务中，直接访问 I2C5、UART3、UART6 和 UART7 等板级外设。启动串口截图 [minicom_output.jpg](assets/minicom_output.jpg) 用于证明系统已经进入板级运行环境，演示视频 [video.mp4](assets/video.mp4) 用于证明语音命令能够驱动机器人动作。

### 5.9 推理输出到控制执行的链路

推理输出先转换为控制侧可理解的动作。例如视觉检测结果可转换为 `STOP`、`MOVE`、`WARN`、`ADJUST` 等控制动作，并附带置信度、目标类别和输入帧编号。智能侧将这些字段封装为任务二定义的协议消息，发送给控制侧。控制侧解析后执行动作，并返回执行状态。

轮足机器人场景中，动作集合被进一步收敛为有限且限幅的运动目标：

| 语音语义 | RT 命令 | 控制目标 |
| --- | --- | --- |
| 前进 | `forward` | 前向速度设为约 `0.3 m/s`，偏航角速度为 0 |
| 后退 | `back` | 前向速度设为约 `-0.3 m/s`，偏航角速度为 0 |
| 左转 | `left` | 偏航角速度设为约 `0.6 rad/s` |
| 右转 | `right` | 偏航角速度设为约 `-0.6 rad/s` |
| 停止 | `stop` | 前向速度和偏航角速度清零 |

实时侧还保留命令 watchdog。方向性命令只在有限窗口内保持，超过保持时间后自动回到 `stop`，避免智能侧卡死、通信丢失或语音未继续输入时机器人持续运动。

完整链路如下：

```text
样例输入/摄像头输入
  -> 智能侧模型推理
  -> 置信度与类别过滤
  -> 控制动作生成
  -> 协议消息发送
  -> 控制侧动作执行
  -> 状态与时间戳回传
  -> 智能侧日志记录
```

### 5.10 轮足实时控制算法

双轮足控制闭环的核心是“状态估计 + LQR 平衡控制 + 运动目标限幅 + 电机协议输出”。`rt-robot` 分支中的控制器以硬件验证过的 ESP32 WBR 控制工程为基础移植，保留确定性的控制数学和协议辅助，去掉 Wi-Fi、阻塞日志和不确定的串口解析路径。

实时控制任务以 8ms 为基础周期，对应 `ESP32_CONTROL_PERIOD_NANOS = 8_000_000`。每个周期内执行以下步骤：

1. 从 RT mailbox 拉取最新语音命令，并转换为 `BalanceTarget`。
2. 通过 I2C5 读取 MPU6050 加速度计和陀螺仪数据。
3. 使用上一周期电机命令和当前 IMU/电机速度反馈运行 EKF，估计 `theta`、`theta_dot`、`velocity`、`yaw_rate`。
4. 根据当前腿部高度计算机体质心、惯量和站立平衡角，形成动力学预测模型。
5. 对不同高度下的 LQR 增益表进行插值，计算左右轮所需扭矩。
6. 对速度、偏航角速度和扭矩做限幅，再转换为 Lingkong 电机 `0xA1` 扭矩控制帧。
7. 通过 UART6/UART3 分别发送右/左轮电机命令，通过 UART7 设置髋部舵机角度。
8. 如果周期超时，记录 deadline miss，并让下一周期从当前时间重新对齐。

控制状态向量和目标向量如下：

| 项目 | 字段 | 含义 |
| --- | --- | --- |
| 估计状态 | `theta` | 机体俯仰角 |
| 估计状态 | `theta_dot` | 俯仰角速度 |
| 估计状态 | `velocity` | 机器人前向速度 |
| 估计状态 | `yaw_rate` | 偏航角速度 |
| 控制目标 | `height` | 轮足腿部目标高度，约束在 `0.07m` 到 `0.20m` |
| 控制目标 | `velocity` | 语音命令映射出的前向速度，限幅在 `±1.0 m/s` |
| 控制目标 | `yaw_rate` | 语音命令映射出的偏航角速度，限幅在 `±1.0 rad/s` |
| 输出约束 | `MAX_TORQUE` | 单轮扭矩限幅，约 `0.75 Nm` |

该算法对实时性的要求来自平衡控制本身，而不是通信协议。语音识别可能需要数百毫秒，属于上层决策输入；一旦命令到达实时侧，机器人保持平衡仍依赖 8ms 闭环持续运行。若智能侧 AI 推理、文件系统、网络或日志负载抢占 Axvisor 预留实时 CPU，或使虚拟化底座在 I/O、调频和中断路径上产生过大抖动，会直接放大控制周期抖动。因此任务一中的 Axvisor 实时 CPU 预留、板级 I/O 路径、调频归因、RT FIFO 调度和 mutex 优先级继承，是让该实物演示稳定运行的必要底座。

### 5.11 状态回传与闭环逻辑

闭环逻辑为每条控制指令建立明确结果。智能侧记录指令序列号、推理时间、发送时间、控制侧接收时间、执行完成时间和响应接收时间。控制侧记录接收序列号、动作类型、执行结果、错误码和当前状态。

通过这些时间戳可以计算模型推理耗时、通信耗时、控制执行耗时和端到端闭环耗时。空载、通信负载、AI 负载和压力负载下的闭环表现共同构成应用层验证结果。

机器人实物演示还应额外记录控制侧周期指标，包括本周期 IMU 读取耗时、控制计算耗时、左右电机 UART 事务耗时、deadline miss 次数和超时纳秒数。对于 8ms 闭环，验收关注点不是 AI 推理是否每 8ms 输出一次，而是控制侧在 AI 负载存在时仍能以 8ms 周期持续执行，并在命令超时后安全降级为停止。

### 5.12 前两项任务对任务三的支撑关系

任务一为任务三提供稳定的 Axvisor AMP 运行环境和实时侧保障，使 AI 推理负载不会直接破坏控制任务；任务二为任务三提供可复现的跨执行域通信协议，使推理结果能够可靠转换为控制动作；任务三则反过来验证任务一和任务二是否真正可用于工控联动场景。

双轮足机器人把这种关系具体化：StarryOS 侧的 SenseVoice/RKNN 推理证明智能侧能力；Axvisor/RT 侧的 8ms 平衡控制证明控制侧实时能力；console/RT mailbox 原型和任务二 GIPC/IP 方案共同说明控制语义可以从智能侧进入控制侧。最终演示视频、语音样例和串口启动截图应作为任务三的交付证据归档，而任务一、任务二的 PR 和测试记录用于解释该演示为什么能够稳定复现。

## 6. 设计实现说明

### 6.1 源码组织

本项目依托 TGOSKits 统一仓库组织代码和测试。比赛相关实现按系统层、通信层和应用层划分：

- 虚拟化底座能力：`os/axvisor/`、`components/axvm`、`components/axvcpu`、`components/axdevice`、`components/axaddrspace` 及架构相关虚拟中断组件。
- 客户机通信能力：客户机应用、网络示例、测试用例或协议库目录。
- AI 联动应用：AI 示例、测试 case 或板级演示目录。
- 交付文档：`project-delivery/quancheng/`。

## 7. 复现与部署说明

### 7.1 构建环境

基础环境参考仓库 `README_CN.md` 和快速上手文档。基础验证入口包括：

```bash
cargo xtask arceos qemu --package ax-helloworld --arch aarch64
cargo xtask starry qemu --arch aarch64
cargo xtask axvisor test qemu --target aarch64
```

### 7.2 Axvisor QEMU 验证路径

Axvisor 场景使用仓库已有测试入口进行基础验证：

```bash
cargo xtask axvisor test qemu --target aarch64
cargo xtask axvisor test qemu --target x86_64
```

手工启动指定 VM 配置时，Axvisor 开发指南中的 Guest 镜像、rootfs 和 `cargo axvisor qemu` 流程作为运行路径。

### 7.3 StarryOS 与客户机能力验证

StarryOS 可用于智能侧客户机能力验证，尤其是 Linux 兼容、网络、脚本和压力负载场景：

```bash
cargo xtask starry test qemu --target aarch64
cargo xtask starry test qemu --target aarch64 --stress
```

## 8. 测试验收关系

测试验收按三任务递进组织：先验收底座，再验收链路，最后验收应用闭环。详细测试项见 [test-plan.md](test-plan.md)。

## 9. 结语

本方案将“Axvisor 实时性与隔离”“控制通信”“AI 联动应用”统一到同一条技术链路中。任务一解决 Axvisor 在 AMP 混合系统中能否稳定运行，任务二解决智能侧与实时侧之间能否可靠交换控制语义，任务三证明前两项能力能够支撑实际 AI 控制闭环。该组织方式既保留每项任务的独立验收证据，也能体现项目整体技术价值。
