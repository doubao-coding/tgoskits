# 泉城比赛项目交付材料

本目录用于集中存放“智能化工控中基于虚拟化的混合系统部署及联动实现”项目的比赛提交材料。材料围绕三项任务组织，但不做简单平铺，而是统一采用“底座 -> 链路 -> 应用”的叙事结构：

- 任务一：实时性与隔离底座，提供虚拟化混合系统稳定运行、资源隔离和关键路径实时性保障。
- 任务二：客户机通信底座，在隔离客户机之间建立可复现、可测试的 IP 通信和应用层协议链路。
- 任务三：AI 联动应用，在前两项基础上完成智能侧推理、控制侧执行和状态回传的闭环展示。

## 材料清单

| 文件 | 用途 |
| --- | --- |
| [technical-solution.md](technical-solution.md) | 设计方案正文，说明总体架构、三任务设计和部署复现关系。 |
| [test-plan.md](test-plan.md) | 测试验收框架，覆盖启动、隔离、通信、性能、AI 联动和综合指标记录。 |
| [pr-summary.md](pr-summary.md) | 参赛 PR 清单，用于整理各 PR 与三项任务之间的对应关系。 |
| [assets/control_voice.wav](assets/control_voice.wav) | 双轮足机器人语音控制输入样例，用于任务三 AI 联动演示。 |
| [assets/video.mp4](assets/video.mp4) | 双轮足机器人端到端实物演示视频。 |
| [assets/minicom_output.jpg](assets/minicom_output.jpg) | 开发板启动 Axvisor/客户机/实时任务的串口输出截图。 |

## 材料关系

三份材料之间的关系如下：

1. `technical-solution.md` 描述总体方案和三任务依赖关系。
2. `test-plan.md` 描述测试维度、记录字段和综合指标。
3. `pr-summary.md` 汇总参赛 PR，并将 PR 映射到任务一、任务二、任务三。

## 与仓库代码的关系

TGOSKits 仓库提供 ArceOS、StarryOS、Axvisor 及相关组件的统一开发与测试入口。本项目交付材料引用的主要代码与配置位置包括：

- `os/axvisor/`：虚拟化运行时、板级配置和 VM 配置，是任务一实时性与隔离底座的主要落点。
- `components/axvm`、`components/axvcpu`、`components/axdevice`、`components/axaddrspace`：虚拟机、vCPU、虚拟设备和地址空间等核心虚拟化组件。
- `components/x86_vlapic`、`components/arm_vgic`：虚拟中断与定时器相关组件，可支撑任务一中的时延与中断路径分析。
- `test-suit/axvisor/`：Axvisor QEMU、U-Boot 与板级测试入口。
- `test-suit/starryos/`：StarryOS 普通测试和压力测试入口，可支撑客户机能力、网络和负载场景验证。
- `drivers/npu/`、`test-suit/starryos/normal/board-orangepi-5-plus/npu-yolov8/`：AI 推理与板级 NPU 验证相关目录，是任务三应用展示的重要参考。
- `rt-robot` 分支中的 `os/axvisor/src/wheel/`、`sensevoice_rknn_npu.py` 和 `orangepi-5-plus-rt-sd-wheel`：双轮足机器人实物演示原型，包含 SenseVoice 语音命令、RT mailbox 转发、8ms 轮足平衡闭环和板级外设控制。
