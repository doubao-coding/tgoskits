# 泉城比赛参赛 PR 清单

## 1. 说明

本文用于汇总本项目参加泉城比赛期间形成的 PR 列表，并说明每个 PR 与赛题三项任务之间的对应关系。三项任务按照“底座 -> 链路 -> 应用”的关系组织：

- 任务一：Axvisor 实时性与隔离底座，支撑混合系统在 AMP 隔离环境中稳定运行。
- 任务二：客户机通信底座，支撑智能侧客户机与控制侧客户机之间可靠通信。
- 任务三：AI 联动应用，基于前两项能力完成推理、控制和状态回传闭环。

## 2. 任务一：Axvisor 实时性与隔离底座 PR

任务一相关 PR 的主线是优化 Axvisor 自身的实时性与隔离能力：先为 Axvisor 建立实时 CPU 预留机制，使智能侧 guest 的 vCPU、虚拟设备和 I/O 负载不能抢占实时控制闭环所需的物理 CPU；再补齐 Axvisor 板级启动、块设备和 RK3588 调频归因能力，降低 guest 加载、存储 I/O 和动态调频对虚拟化底座实时路径的扰动。RT FIFO 和 mutex priority inheritance 作为实时任务语义的支撑组件记录，用于说明预留 CPU 上的实时任务具备高优先级运行和锁等待抑制能力，但任务一的主要优化对象是 Axvisor 运行时本身。

| PR 编号/链接 | PR 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| [#2160](https://github.com/rcore-os/tgoskits/pull/2160) | `feat(axvisor): reserve realtime CPU` | 为 Axvisor 增加实时 CPU 预留与运行时识别能力，使虚拟化底座能够把实时控制路径与智能侧 guest vCPU、虚拟设备和普通负载隔离开 | `os/axvisor/`、`os/arceos/modules/axruntime/`、`virtualization/axvm/`、`docs/design/axvisor-realtime-cpu.md` | Axvisor 构建路径、VM host 配置和实时 CPU 运行时记录验证 | 已提交到 `dev` |
| [#2161](https://github.com/rcore-os/tgoskits/pull/2161) | `feat(ax-sched): add single-core RT FIFO scheduler` | 新增 `RtFifoScheduler`、`sched-rt-fifo` feature 和调度器设计文档，作为预留实时 CPU 上控制任务的高优先级运行语义支撑 | `components/axsched/`、`os/arceos/modules/axtask/`、`test-suit/arceos/rust/`、`docs/design/current-fifo-issues-and-improvements.md` | `cargo xtask arceos test qemu --test-group rust --test-case sched-rt-fifo --target x86_64-unknown-none`，调度器单测和 QEMU 成功 regex | 已提交到 `dev` |
| [#2162](https://github.com/rcore-os/tgoskits/pull/2162) | `feat(axtask): add mutex priority inheritance` | 基于 #2161 的 RT FIFO 调度补齐 mutex 优先级继承，降低实时任务在锁等待中被普通任务间接阻塞的风险 | `os/arceos/modules/axtask/`、`components/axsched/`、`test-suit/arceos/rust/src/task/rt_fifo.rs`、`docs/design/axtask-priority-inheritance.md` | `cargo xtask clippy --package ax-task`、`cargo xtask clippy --package ax-sched`、`sched-rt-fifo` QEMU PI 场景验证 | 已提交到 `dev`，依赖 #2161 先合入 |
| [#2163](https://github.com/rcore-os/tgoskits/pull/2163) | `feat(ax-driver): add IRQ-driven virtio-blk block controller and Axvisor QEMU starry guest smoke` | 新增 IRQ 驱动的 virtio-blk 块设备控制器（描述符表、一致性 DMA、队列所有权契约），并配套 Axvisor QEMU aarch64 starry guest 配置与 smoke 测试用例，支撑 guest 从 virtio-blk 根盘启动 | `drivers/ax-driver/src/virtio/`、`os/axvisor/configs/`、`test-suit/axvisor/` | ax-driver 全 feature clippy；QEMU smoke 用例启动 guest 并命中成功标志 | 已提交到 `dev` |
| [#2164](https://github.com/rcore-os/tgoskits/pull/2164) | `fix(axvisor): enable rockchip-dwmmc on OrangePi 5 Plus` | 板级与测试构建配置启用 `ax-driver/rockchip-dwmmc`：SD 卡位于 mmc@fe2c0000 的 DW 主机之后，缺该 feature 时 axvisor 无法从 fs 加载 guest 镜像 | `os/axvisor/configs/board/`、`test-suit/axvisor/normal/board-orangepi-5-plus/` | axvisor 板级构建；板上 SD 枚举与 guest 内核从 ext4 加载 | 已提交到 `dev` |
| [#2165](https://github.com/rcore-os/tgoskits/pull/2165) | `fix(ax-driver): attribute RK3588 governor busy by FDT cpu topology` | 调频 governor 的 busy 归因改为按 guest FDT `/cpus` 节点的 SCMI clock id 映射逻辑 CPU 到实际簇；无在线 CPU 的簇不再被误降频；降档设 boot OPP 地板；新增只读频率 readout | `drivers/ax-driver/src/soc/rockchip/` | 归因单测（clock id 表、物理兜底分区、单大核 guest 映射）；板上 gov 日志与频率 readout 验证 | PR open，base `dev` |
## 3. 任务二：客户机通信底座 PR

任务二相关提交和 PR 主要体现客户机之间的通信能力，包括虚拟网络配置、IP 链路、应用层协议、请求响应、心跳、超时、重试和异常处理。前三项为通信链路的底层前置提交，后五项为直接实现通信能力的 PR。

| PR 编号/链接 | PR 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| [#1926](https://github.com/rcore-os/tgoskits/pull/1926) | `fix(axvm): preserve PSCI in generated guest FDT` | 修复 guest FDT 中 PSCI 信息丢失，保证 StarryOS/ArceOS 客户机稳定启动 | `virtualization/axvm/` | guest FDT、启动和 vCPU 路径验证 | 已合并 |
| [#1927](https://github.com/rcore-os/tgoskits/pull/1927) | `feat(axvisor): add dual-guest virtio-net support` | 增加双 guest VirtIO-net、MAC 配置和 AxVisor 进程内二层交换 | `os/axvisor/`、VirtIO-net 设备和交换路径 | 双 guest VirtIO-net 拓扑和设备图验证 | 已合并 |
| [#1935](https://github.com/rcore-os/tgoskits/pull/1935) | `feat(axvirtio-blk): add virtio-mmio block device core` | 增加 VirtIO-MMIO 块设备核心，为客户机镜像和虚拟设备运行提供基础 | `components/axvirtio-blk/`、VirtIO-MMIO 公共组件 | VirtIO-MMIO 设备核心构建和设备接入验证 | 已合并 |
| [#2155](https://github.com/rcore-os/tgoskits/pull/2155) | `feat(axvisor): connect StarryOS and ArceOS guests over virtio-net` | 配置 StarryOS 与 ArceOS 双 guest VirtIO-net、MAC 地址和内部二层交换拓扑 | `os/axvisor/`、`os/axvisor/configs/` | AxVisor QEMU 双 guest 配置和网络拓扑验证 | 已提交到 `dev` |
| [#2156](https://github.com/rcore-os/tgoskits/pull/2156) | `feat(net-protocol): add StarryOS and ArceOS guest control protocol` | 实现包含版本、消息类型、长度、序列号、时间戳、错误码和 CRC 的应用层帧，以及 CONTROL/STATUS/ERROR/HEARTBEAT/ACK 消息 | `components/guest-ip-protocol/`、`apps/starry/guest-ip-link/`、`apps/arceos/guest-ip-server/` | 协议编解码、长度/版本/校验错误测试和两端构建检查 | 已提交到 `dev` |
| [#2157](https://github.com/rcore-os/tgoskits/pull/2157) | `feat(net-reliability): recover StarryOS and ArceOS guest sessions` | 增加 TCP 分帧、超时、有限重试、断连重连、递增序列号、重复/乱序识别和异常恢复 | `components/guest-ip-protocol/`、两端 guest endpoint | 可靠性单测、ArceOS Clippy 和客户端断连恢复测试 | 已提交到 `dev` |
| [#2158](https://github.com/rcore-os/tgoskits/pull/2158) | `test(net): validate StarryOS and ArceOS guest communication` | 增加 StarryOS/ArceOS GIPC QEMU 拓扑、启动脚本、客户端注入、成功标志和指标聚合 | `os/axvisor/configs/`、`os/axvisor/scripts/`、`scripts/test/guest-ip-link/` | QEMU 流程、ready/status/metric 标志、成功率和 RTT/吞吐统计 | 已提交到 `dev` |
| [#2159](https://github.com/rcore-os/tgoskits/pull/2159) | `fix(net): complete StarryOS guest bootstrap and link observability` | 配置 StarryOS `eth0` 为 `10.0.42.1/24`，支持多请求序列、ERROR 解析、应用错误/超时/重连/恢复指标和 RTT P50/P95 | `apps/starry/guest-ip-link/`、`scripts/test/guest-ip-link/`、`docs/design/starry-rtos-ip-link.md` | 协议测试、ArceOS 构建与 Clippy、客户端恢复和指标脚本检查 | 已提交到 `dev` |

## 4. 任务三：AI 联动应用 PR

任务三相关 PR/提交主要体现 AI 与控制联动的应用闭环。除已明确编号的 PR 外，`xiaohui` 在 `origin/xiaohui/sensevoice` 上已经提交过 SenseVoice QEMU 应用、Axvisor+StarryOS 端到端用例、OrangePi 5 Plus RK3588 NPU 板级适配、启动注入和下载稳定性修复；这些内容先以 commit 形式记录，后续如有 PR 编号再补齐。

| PR/提交 | 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| [`672793b95`](https://github.com/rcore-os/tgoskits/commit/672793b9572855a3bd7b795c1151c8490aad4542) | `feat(starry-apps): add sensevoice QEMU app running SenseVoice ASR on StarryOS` | 增加 StarryOS QEMU SenseVoice ASR 应用，准备 sherpa-onnx、glibc runtime、模型、tokens 和 zh/en 样例音频 | `apps/starry/sensevoice/` | QEMU aarch64 guest 内运行 L0-L3 测试，zh/en 转写匹配参考文本 | 已提交到 `origin/xiaohui/sensevoice` |
| [`a1444c0ee`](https://github.com/rcore-os/tgoskits/commit/a1444c0ee68496d1db27e35a1557277667ba711c) | `feat(axvisor): add end-to-end SenseVoice ASR case for the QEMU starry guest` | 将 Axvisor、StarryOS guest、virtio-blk rootfs 和 SenseVoice 测试串成端到端 QEMU 用例 | `test-suit/axvisor/normal/qemu-starry/sensevoice/` | Axvisor QEMU sensevoice case PASS，记录 hypervisor 层 RTF 开销 | 已提交到 `origin/xiaohui/sensevoice` |
| [`7d292062c`](https://github.com/rcore-os/tgoskits/commit/7d292062c9a17c6318c3f294e59252a1f217f81d) | `feat(starry-apps): add sensevoice-rknn board app skeleton for OrangePi 5 Plus` | 增加 OrangePi 5 Plus RK3588 NPU SenseVoice 板级应用骨架，包含 RKNN runtime 调用、fbank/LFR/CMVN/CTC、测试脚本和 host frontend 数值检查 | `apps/starry/sensevoice-rknn/` | host frontend 数值对拍；板级 runbook 记录固定 shape、driver/runtime 风险点 | 已提交到 `origin/xiaohui/sensevoice` |
| [`54ad820b3`](https://github.com/rcore-os/tgoskits/commit/54ad820b3f5484d8f4f46586c6136f9c9c5ed06c) | `feat(sensevoice-rknn): wire axvisor+starry+NPU path on OrangePi 5 Plus` | 打通 Axvisor -> StarryOS guest -> RK3588 NPU passthrough -> librknnrt 的板级链路，修正 tensor attr、batch concat、输出 reshape 和 guest 内存配置 | `apps/starry/sensevoice-rknn/`、`os/axvisor/configs/board/`、`os/axvisor/configs/vms/`、`test-suit/axvisor/normal/board-orangepi-5-plus/` | 板上运行到 `rknn_init/run/outputs`，AXVISOR-BOARD-RUNBOOK 记录交付步骤和剩余风险 | 已提交到 `origin/xiaohui/sensevoice` |
| [#2166](https://github.com/rcore-os/tgoskits/pull/2166) | `perf(rk3588): close the guest performance gap on OrangePi 5 Plus` | 模型性能优化：guest vCPU 绑定 A76 大核、板级日志降噪、card1 ioctl 聚合计时、readahead 窗口扩至 1 MiB，并配合 #2165 的 RK3588 governor 归因修复收敛性能差距 | `os/axvisor/configs/vms/`、`os/StarryOS/kernel/src/pseudofs/dev/`、`fs/ax-fs-ng/src/file/cache/` | 板级 `[perf]` 日志；推理 2.96s→1.72s，叠加动态调频后 1.46s；NPU submit 7.78ms/次接近原生 | PR open，base `dev` |
| [`58cb3b197`](https://github.com/rcore-os/tgoskits/commit/58cb3b197eaf3f9eb00977939fbb05d39ad35acf) | `fix(axvisor): automate starry guest kernel load and fail fast on hung guests` | 应用启动优化：Axvisor QEMU Starry guest 改为构建期嵌入 kernel，健康启动失败时快速失败，避免手工注入和长时间挂起 | `os/axvisor/configs/vms/qemu/aarch64/`、`test-suit/axvisor/normal/qemu-starry/` | smoke PASS，sensevoice PASS，并对 EL2 异常循环设置失败判定 | 已提交到 `origin/xiaohui/sensevoice` |
| [`2b92958ff`](https://github.com/rcore-os/tgoskits/commit/2b92958ff8254381eca63a6ab692ef18ed58cd18) | `fix(axbuild): inject overlay via cd+basename to debugfs-resolvable paths` | 应用启动优化：修复 rootfs overlay 注入路径，避免 debugfs 将绝对路径写成不可解析文件名；放宽不可复现 glibc arm64 SHA gate | `scripts/axbuild/src/rootfs/inject.rs`、`apps/starry/sensevoice/prebuild.sh` | `cargo xtask starry app qemu -t sensevoice --arch aarch64` 到达 `SENSEVOICE_TEST_PASSED` | 已提交到 `origin/xiaohui/sensevoice` |
| [`e325b5aa2`](https://github.com/rcore-os/tgoskits/commit/e325b5aa2e7234088135b85cc039b572c88883f5) / [`b0022043f`](https://github.com/rcore-os/tgoskits/commit/b0022043f5ec18e3de6c8616c42d1c31e27ef19a) | SenseVoice 资产下载稳定性修复 | 应用启动优化：为模型、runtime、样例音频下载增加 mirror fallback、断点续传和指数退避重试，降低复现环境网络波动影响 | `apps/starry/sensevoice/prebuild.sh` | 受限网络和 transient TLS EOF 场景下 prebuild 可恢复 | 已提交到 `origin/xiaohui/sensevoice` |

## 5. 测试验收与文档 PR

除功能实现 PR 外，项目还需要整理测试验收和文档交付类 PR，用于证明方案可复现、可检查、可评审。

| PR 编号/链接 | PR 标题 | 类型 | 主要内容 | 对应材料 |
| --- | --- | --- | --- | --- |
| 待补 | 待补 | 测试验收 | 启动、隔离、通信、AI 闭环和端到端时延测试 | `test-plan.md` |
| 待补 | 待补 | 文档交付 | 技术方案、测试文档、PR 清单和复现说明 | `project-delivery/quancheng/` |

## 6. 待补材料

正式提交前需要补齐以下内容：

- 每个参赛 PR 的编号、链接、标题和合并状态。
- 每个 PR 的主要修改内容和涉及目录。
- 每个 PR 对应的测试命令或验证记录。
- 任务三端到端演示日志、截图、视频或时延统计表。
