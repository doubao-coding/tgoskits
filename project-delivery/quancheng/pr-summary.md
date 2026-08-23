# 泉城比赛参赛 PR 与基础提交清单

## 1. 项目归属

以下 8 项共同组成“任务二：客户机通信底座”的实现链路，目标是在 Axvisor 管理的 StarryOS/Linux 客户机与 ArceOS/RTOS 客户机之间建立基于 IP 的双向通信，并提供应用层协议、可靠性、验证和可观测性。

前三项是通信链路的底层前置提交，后五项是直接面向客户机通信的 PR。它们共同形成：

```text
guest 启动与设备基础
  -> 双 guest VirtIO-net 与内部交换
  -> IP 传输
  -> 应用层协议
  -> TCP 可靠性与恢复
  -> QEMU 验证与指标
  -> StarryOS 网络启动和可观测性补齐
```

## 2. 任务二：底层前置提交

| 提交 | 标题 | 主要修改内容 | 涉及目录 | 与通信链路的关系 |
| --- | --- | --- | --- | --- |
| [`f56b496fe`](https://github.com/rcore-os/tgoskits/commit/f56b496fe) | `fix(axvm): preserve PSCI in generated guest FDT` | 修复生成 guest FDT 时 PSCI 信息丢失，保证客户机启动和 vCPU 电源管理路径稳定 | `virtualization/axvm/`、guest FDT 生成逻辑 | 保证 StarryOS/ArceOS guest 能够稳定启动，是通信测试的启动前提 |
| [`547f3266a`](https://github.com/rcore-os/tgoskits/commit/547f3266a) | `feat(axvisor): add dual-guest virtio-net support (#1927)` | 增加双 guest VirtIO-net 设备、MAC 配置和 AxVisor 进程内二层交换能力 | `os/axvisor/`、VirtIO-net 设备和交换路径 | 提供两个客户机之间 IP 数据包传输的底层网络通道 |
| [`4c4c55cd2`](https://github.com/rcore-os/tgoskits/commit/4c4c55cd2) | `feat(axvirtio-blk): add virtio-mmio block device core (#1935)` | 增加 VirtIO-MMIO 块设备核心和公共设备基础 | `components/axvirtio-blk/`、VirtIO-MMIO 公共组件 | 提供客户机虚拟设备接入和镜像运行基础，通信业务不使用块设备作为数据通道 |

## 3. 任务二：客户机通信 PR

| PR | 标题 | 主要修改内容 | 涉及目录 | 测试或验证方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| [#2155](https://github.com/rcore-os/tgoskits/pull/2155) | `feat(axvisor): connect StarryOS and ArceOS guests over virtio-net` | 配置 StarryOS 与 ArceOS 双 guest VirtIO-net、MAC 地址和 AxVisor 内部二层交换拓扑；提供 AArch64 QEMU VM/board 配置 | `os/axvisor/`、`os/axvisor/configs/`、`virtualization/axvm/` | AxVisor QEMU 双 guest 启动配置、VirtIO-net 设备图和网络拓扑检查 | 已提交到 `dev` |
| [#2156](https://github.com/rcore-os/tgoskits/pull/2156) | `feat(net-protocol): add StarryOS and ArceOS guest control protocol` | 实现固定版本应用层帧，包含 magic、版本、消息类型、flags、头/载荷长度、序列号、时间戳、错误码和 CRC；实现 CONTROL、STATUS、ERROR、HEARTBEAT 和 ACK 语义 | `components/guest-ip-protocol/`、`apps/starry/guest-ip-link/`、`apps/arceos/guest-ip-server/` | 协议编解码、长度/版本/校验错误测试；StarryOS C 客户端与 ArceOS Rust 服务端构建检查 | 已提交到 `dev` |
| [#2157](https://github.com/rcore-os/tgoskits/pull/2157) | `feat(net-reliability): recover StarryOS and ArceOS guest sessions` | 增加 TCP 分帧、连接/读写超时、有限重试、断连重连、递增序列号、重复/乱序识别和异常恢复 | `components/guest-ip-protocol/`、两端 guest endpoint | 协议可靠性单测、ArceOS 构建与 Clippy、客户端断连恢复测试 | 已提交到 `dev` |
| [#2158](https://github.com/rcore-os/tgoskits/pull/2158) | `test(net): validate StarryOS and ArceOS guest communication` | 增加 StarryOS/ArceOS GIPC QEMU 拓扑、启动脚本、rootfs 客户端注入、成功标志校验和指标聚合 | `os/axvisor/configs/`、`os/axvisor/scripts/`、`scripts/test/guest-ip-link/` | QEMU 启动流程、`GIPC_RTOS_READY`/`GIPC_STARRY_STATUS`/`GIPC_STARRY_METRIC` 标志、成功率和 RTT/吞吐统计 | 已提交到 `dev` |
| [#2159](https://github.com/rcore-os/tgoskits/pull/2159) | `fix(net): complete StarryOS guest bootstrap and link observability` | 配置 StarryOS `eth0` 为 `10.0.42.1/24`；支持多请求递增序列、ERROR 帧解析、应用错误/超时/重连/恢复指标和 RTT P50/P95 聚合 | `apps/starry/guest-ip-link/`、`scripts/test/guest-ip-link/`、`docs/design/starry-rtos-ip-link.md` | 协议测试、ArceOS 构建与 Clippy、客户端断连恢复、指标脚本和格式检查 | 已提交到 `dev` |

## 4. 网络链路验收口径

这组提交对应的默认通信拓扑为：

```text
StarryOS guest                         ArceOS guest
VirtIO-net                              VirtIO-net
MAC 52:54:00:42:00:01                  MAC 52:54:00:42:00:02
10.0.42.1/24                            10.0.42.2/24
          \\                            /
           AxVisor 进程内二层交换机
```

- 主数据通道：TCP/IP，服务端口 `4242`。
- StarryOS 侧：`eth0 = 10.0.42.1/24`。
- ArceOS 侧：`eth0 = 10.0.42.2/24`。
- 不使用共享内存、HyperCall、裸 MMIO 或 vsock 承载业务数据。
- 应用层指标包括请求成功率、应用错误、超时、重连/恢复次数、RTT P50/P95 和有效吞吐量。

## 5. 与其他任务的边界

这 8 项统一归入任务二，是任务三 AI 联动应用的通信基础。它们本身不包含 AI 模型推理、推理结果到控制动作的业务映射或具体板级 AI 演示；任务三应在此通信底座上另行提供应用闭环 PR 和实测材料。

## 6. 待补充的交付证据

正式比赛材料仍应为每个 PR 附上最终合并状态、对应 CI/QEMU 日志链接，以及任务三的 AI 推理、控制执行和端到端闭环时延记录。该清单负责固定 8 项提交与任务二的映射关系。
