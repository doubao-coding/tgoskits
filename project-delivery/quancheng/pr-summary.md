# 泉城比赛参赛 PR 清单

## 1. 说明

本文用于汇总本项目参加泉城比赛期间形成的 PR 列表，并说明每个 PR 与赛题三项任务之间的对应关系。三项任务按照“底座 -> 链路 -> 应用”的关系组织：

- 任务一：实时性与隔离底座，支撑混合系统在虚拟化环境中稳定运行。
- 任务二：客户机通信底座，支撑智能侧客户机与控制侧客户机之间可靠通信。
- 任务三：AI 联动应用，基于前两项能力完成推理、控制和状态回传闭环。

## 2. 任务一：实时性与隔离底座 PR

任务一相关 PR 主要体现虚拟化混合系统的底座能力，包括客户机资源配置、VM 启动、vCPU 管理、内存隔离、设备隔离、中断/定时器路径和实时性测试。

| PR 编号/链接 | PR 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 待补 | 待补 | 待补 | `os/axvisor/`、`components/`、`test-suit/axvisor/` | 待补 | 待补 |

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

任务三相关 PR 主要体现 AI 与控制联动的应用闭环，包括模型推理、推理结果到控制动作的转换、控制指令下发、控制侧执行、状态回传和端到端时延统计。

| PR 编号/链接 | PR 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 待补 | 待补 | 待补 | `drivers/npu/`、AI 示例、板级测试或应用演示目录 | 待补 | 待补 |

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
