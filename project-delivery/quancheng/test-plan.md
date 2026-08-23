# 泉城比赛测试验收框架

## 1. 测试目标

测试验收围绕“底座 -> 链路 -> 应用”的技术关系展开，用于支撑三项任务的功能验证、性能记录和最终材料汇总。

- 任务一关注 Axvisor 虚拟化底座下的多客户机启动、资源隔离、实时性和稳定性。
- 任务二关注智能侧客户机与控制侧客户机之间的通信链路、协议语义、可靠性和异常处理。
- 任务三关注 AI 推理结果到控制动作的转换、控制侧执行、状态回传和端到端闭环时延。

## 2. 测试环境

| 项目 | 内容 |
| --- | --- |
| 仓库 | `tgoskits` |
| 基础系统 | Axvisor、StarryOS、ArceOS |
| 智能侧客户机 | Starry 生态 / Linux 兼容运行环境 |
| 控制侧客户机 | 实时优化 ArceOS / RTOS 运行环境 |
| 仿真环境 | QEMU AArch64、x86_64、RISC-V、LoongArch64 |
| 板级环境 | Orange Pi 5 Plus、Roc RK3568 PC、RDK S100 等 |
| 测试目录 | `test-suit/arceos/`、`test-suit/starryos/`、`test-suit/axvisor/` |

## 3. 任务一：实时性与隔离底座测试

任务一测试用于观察虚拟化底座在多客户机场景下的启动、隔离和实时运行情况。

| 测试维度 | 关注内容 | 记录材料 |
| --- | --- | --- |
| 启动与部署 | Axvisor 启动、客户机镜像加载、VM 配置解析、多客户机组合运行 | 启动日志、VM 配置、串口输出 |
| 资源隔离 | CPU、内存、设备访问边界，智能侧负载对控制侧的影响 | 配置说明、压力负载日志、异常记录 |
| 实时性 | 控制侧周期任务延迟、中断响应、定时器路径、vCPU 调度影响；双轮足机器人关注 8ms 平衡控制周期是否持续满足 | 平均延迟、P95/P99、最大延迟、deadline miss 次数 |
| 稳定性 | 长时间运行、客户机异常、压力负载下底座行为 | 长稳日志、panic/错误统计 |

### 3.1 实测记录

| 场景 | 平台 | 命令或用例 | 关键结果 | 日志位置 | 备注 |
| --- | --- | --- | --- | --- | --- |
| Axvisor QEMU 启动 | 待补 | 待补 | 待补 | 待补 | 任务一 |
| 多客户机组合启动 | 待补 | 待补 | 待补 | 待补 | 任务一 |
| 控制侧周期任务延迟 | 待补 | 待补 | 待补 | 待补 | 任务一 |
| 智能侧压力负载干扰 | 待补 | 待补 | 待补 | 待补 | 任务一 |
| 双轮足 8ms 控制闭环 | Orange Pi 5 Plus + 双轮足机器人 | `rt-robot` 分支实物配置 | RT 侧持续执行 8ms balance loop，记录 deadline miss 和电机/IMU 耗时 | `assets/minicom_output.jpg`、运行串口日志 | 任务一/任务三 |

## 4. 任务二：客户机通信底座测试

任务二测试验证从 guest 启动、VirtIO-net 二层数据面、IPv4/TCP 连接，到 GIPC 控制请求、状态响应、错误通知、断连恢复和指标归档的完整证据链。测试按“静态质量 → 协议单元 → 确定性故障注入 → StarryOS/ArceOS 双 guest 端到端 → 日志与指标验收”逐层推进；下层通过不能替代上层运行证据。

![任务二客户机通信测试证据金字塔](assets/gipc-test-pyramid.svg)

### 4.1 测试对象、追溯范围与通过原则

| 被测层 | 对应 PR | 主要对象 | 关键不变量 |
| --- | --- | --- | --- |
| guest 启动与设备基础 | #1926、#1935 | PSCI/FDT、VirtIO-MMIO、镜像/rootfs | 两个 guest 可启动并发现设备；块设备不承载通信 payload |
| 二层网络 | #1927、#2155 | 双 VirtIO-net、Axvisor VirtualSwitch、MAC/IRQ/DMA | 固定 MAC、ARP/单播双向可达、源 MAC anti-spoof、无业务旁路 |
| 应用协议 | #2156 | `guest-ip-protocol`、Starry C client、ArceOS Rust server | 32 B 固定头、≤1200 B payload、大端序、CRC、同 sequence 响应 |
| 可靠性 | #2157 | TCP framing、retry、sequence window、accept recovery | 传输失败不伪造成功，只有合法 STATUS 完成请求 |
| 运行与指标 | #2158、#2159 | QEMU runner、network-init、verify/aggregate | 静态地址真实生效，错误非零传播，指标可复核 |

通过判定分为两级：QEMU 在线 regex 只承担快速冒烟；正式交付必须再检查网络和 listener marker、逐请求 sequence、完整 metric、离线 verifier/aggregator 及原始日志。正常基线要求全部请求成功且无错误；故障注入允许出现预期 timeout/recoverable marker，但必须满足场景规定的恢复或拒绝结果。

### 4.2 环境、拓扑与测试数据固定

| 项目 | 固定配置/记录要求 |
| --- | --- |
| QEMU | AArch64 `virt,virtualization=on,gic-version=3`、`cortex-a72`、4 vCPU、4 GiB、`-nographic`、180 秒门限 |
| StarryOS VM | VM 1、pCPU 1、1 GiB、MAC `52:54:00:42:00:01`、`eth0=10.0.42.1/24` |
| ArceOS VM | VM 2、pCPU 2、512 MiB、MAC `52:54:00:42:00:02`、`eth0=10.0.42.2/24`、TCP 4242 |
| 网络边界 | Axvisor 进程内 L2 switch；默认无 host bridge、TAP、NAT、uplink 或默认网关 |
| GIPC | version=1、header=32 B、max payload=1200 B、CONTROL payload=8 B |
| 运行产物 | commit、QEMU/工具链版本、rootfs SHA-256、VM 配置、完整串口日志、命令退出码 |

端到端运行前后均应保存 StarryOS 的 `ip addr show dev eth0` 与 `ip route show dev eth0`，并核对两个 VM TOML 中的固定 MAC。`ping` 只能作为 ARP/ICMP 辅助证据，不能替代 TCP CONTROL→STATUS 业务闭环。

### 4.3 分层执行命令

```bash
# 协议单元与静态质量
cargo test -p guest-ip-protocol
cargo clippy -p guest-ip-protocol --all-targets -- -D warnings
cargo check -p arceos-guest-ip-server --no-default-features
cargo clippy -p arceos-guest-ip-server \
  --no-default-features --all-targets -- -D warnings
cc -std=c11 -Wall -Wextra -Werror -O2 \
  apps/starry/guest-ip-link/linux-client.c -o /tmp/gipc-starry-client
python3 -m py_compile scripts/test/guest-ip-link/*.py

# 确定性客户端断连恢复
python3 scripts/test/guest-ip-link/test_linux_client.py

# 双 guest 端到端运行与离线验收
ROOTFS_IMAGE=<starry-rootfs.ext4> \
  os/axvisor/scripts/run-qemu-aarch64-starry-rtos-gipc.sh 2>&1 | tee <guest.log>
python3 scripts/test/guest-ip-link/verify_metrics.py <guest.log>
python3 scripts/test/guest-ip-link/aggregate_metrics.py <guest.log>
```

### 4.4 协议单元与互操作测试

仓库现有 `cargo test -p guest-ip-protocol` 包含以下确定性用例：

| ID | 现有用例 | 输入 | 精确期望 | 证明范围 |
| --- | --- | --- | --- | --- |
| P-A01 | `round_trip_preserves_header_and_payload` | CONTROL、ACK_REQUIRED、payload=`hello`、seq=7、timestamp=99 | encode/decode 后类型、序列、时间戳和 payload 保持 | Rust 侧 wire round-trip |
| P-A02 | `rejects_corrupted_payload` | 合法 STATUS 后翻转 payload bit | `ChecksumMismatch` | CRC 覆盖 payload |
| P-A03 | `rejects_trailing_bytes` | 合法空 HEARTBEAT 后追加一字节 | `PayloadLengthMismatch` | 完整长度必须等于 32+payload_len |
| R-A01 | `retries_then_times_out_with_a_bounded_budget` | timeout=10 ms、max_retries=1，在 109/110/119/120 ms 轮询 | Wait→Retransmit→Wait→TimedOut | 纯 RetryPolicy 状态机，不等于 C socket attempt |
| R-A02 | `duplicate_and_out_of_order_requests_are_not_new_work` | observe 10、10、9、11 | New、Duplicate、OutOfOrder、New | 连接内序列分类 |

需要用 raw-frame harness 补充并归档的协议专项如下。这些是测试计划，不得写成现有五个单测已经覆盖：

| ID | 注入 | 预期 |
| --- | --- | --- |
| P-F01 | bad magic/version/header_len/unknown type/error code | ArceOS fail-closed 结束当前连接并记录 recoverable error；下一合法连接仍可成功 |
| P-F02 | `payload_len=1201` 或 payload 截断 | 在无界分配前拒绝；EOF 后返回 accept |
| P-F03 | CONTROL CRC 翻转 | `ChecksumMismatch`、连接关闭、后续连接恢复 |
| P-F04 | CONTROL payload 长度 7/9 且 CRC 合法 | 同序列 ERROR `InvalidPayload=5`，连接保持可用 |
| P-F05 | 把 STATUS/ERROR 作为请求 | 同序列 ERROR `UnsupportedMessage=6` |
| P-F06 | HELLO/HEARTBEAT，payload 0..1200 B | 同序列、同 payload STATUS；当前标准 C client 不发送这两类消息 |
| P-F07 | C 端 40 B CONTROL → Rust server | 逐字段验证 STATUS 的 magic/version/length/sequence/CRC，证明 C/Rust 互操作 |
| P-F08 | ACK 请求 | 当前 server 不响应；证明 ACK 是 UDP/扩展预留，不是 TCP 成功门槛 |

### 4.5 网络启动、二层与端到端控制闭环

正常基线按照以下顺序核验，但两个 guest 并发启动时 NET_READY 与 LISTEN 的相对顺序可变化：

1. `GIPC_RTOS_READY`：应用入口已执行；不能单独证明 listener ready。
2. `GIPC_RTOS_LISTEN ip=10.0.42.2 port=4242`：ArceOS IP 配置与 bind 成功。
3. `GIPC_STARRY_NET_READY interface=eth0 address=10.0.42.1/24 peer=10.0.42.2`：StarryOS 接口和地址 ready；另保存路由输出。
4. `GIPC_RTOS_CONNECTED peer=10.0.42.1:<ephemeral>`：TCP accept 成功。
5. `GIPC_STARRY_STATUS seq=N payload=8 ...`：一次 CONTROL→STATUS 完整闭环。
6. `GIPC_STARRY_METRIC`：进程级结果；随后 verifier 输出 `GIPC_METRICS_OK`，aggregator 输出 `GIPC_AGGREGATE`。

| ID | 场景 | 操作 | 正常基线通过条件 |
| --- | --- | --- | --- |
| E01 | 网络与 listener | 启动双 guest，保存地址/路由/marker | MAC/IP/路由与配置一致；无 NET_ERROR/RTOS_ERROR |
| E02 | 单请求 CONTROL | `/usr/bin/gipc-starry-client 10.0.42.2 1` | STATUS seq=1；requests=success=1；errors/timeouts/reconnects=0；RTT/吞吐>0 |
| E03 | 多请求 sequence | `/usr/bin/gipc-starry-client 10.0.42.2 100` | STATUS seq=1..100 各一次；requests=success=100；errors=0 |
| E04 | L2 行为 | 观察 ARP、固定 MAC 和 switch counter/日志 | ARP 广播到对端；已知单播不泛洪；伪造源 MAC 被丢弃 |

多请求模式中客户端每个请求关闭并重建 TCP，服务端也为每个连接重建 sequence window。因此 E03 证明进程内序号递增和响应关联，不证明跨连接去重或 exactly-once。

### 4.6 故障注入与恢复矩阵

![GIPC 故障注入、恢复边界与验收观测面](assets/gipc-fault-observability.svg)

| ID | 故障与注入方法 | 当前预期行为 | 必查证据 | 覆盖状态 |
| --- | --- | --- | --- | --- |
| F01 | `GIPC_INTERFACE=missing0` | NET_ERROR、退出 1、客户端不启动 | 无 STATUS，非零退出 | 需脚本专项/人工 guest |
| F02 | 目标端口无 listener | 每请求最多 3 attempt，耗尽后 TIMEOUT | attempts=3、success=0、进程退出1 | 行为已实现，需确定性用例 |
| F03 | 首次 accept 后立即 close，第二次正常 | 同 sequence 重连并最终 STATUS | attempts=2、timeouts=1、reconnects=1、recovery=1 | 已有 host 自动测试；后两字段尚未显式 assert |
| F04 | accept 后静默超过接收 timeout | 客户端关闭并重试；三次静默则 TIMEOUT | timeout 计数和总预算 | 需 mock 测试 |
| F05 | 把响应头/payload 拆为 1–3 B 片段 | `read_full` 重组，不提前解析 | attempts=1、errors/timeouts=0 | 代码支持，需强制短读用例 |
| F06 | 响应 CRC/version/sequence/type 错误 | STARRY_ERROR，errors=1，不重试协议失败 | attempts=1、退出1 | 需 C client mock 测试 |
| F07 | 合法 ERROR code=5 | 打印 code/seq，errors=1，不计 success | 非零退出 | 需 mock 测试 |
| F08 | 同连接重复 CONTROL seq=10 | 第二帧分类 Duplicate，返回同序列/同 payload STATUS，不进入 New 分派 | 两个 STATUS；不宣称持久 exactly-once | Rust 分类已测，socket 路径待测 |
| F09 | 同连接先 seq=10 后 seq=9 | seq=9 返回 ERROR `InvalidSequence=4`，窗口不推进 | STATUS 10、ERROR 9、随后 seq11 可成功 | 分类已测，线上路径待测 |
| F10 | 坏帧连接后再建合法连接 | RECOVERABLE_ERROR 后 server 回到 accept | RECOVERABLE_ERROR→CONNECTED→STATUS | 需 server/双 guest 专项 |
| F11 | ArceOS 服务或 guest 重启 | 仅在客户端 3 attempt 窗口内可恢复 | 第二次 READY/LISTEN、STATUS、recovery=1 | 人工端到端 |
| F12 | 连接后只发 1–31 B 且不关闭 | 当前 server 无 read/idle deadline，串行服务可能被占用 | 记录为已知风险，不标“恢复通过” | 已知能力缺口 |

客户端 `timeouts` 实际合并 connect/send/read/payload 失败和长度超限，准确含义是“传输失败/超时 attempt 数”；协议错误和 ERROR 立即失败，不进入三次传输重试。服务端的重复/乱序只在当前连接内有效。测试报告必须区分“连接级解析失败后继续 accept”“客户端有限重连”和“整个 RTOS guest 重启恢复”三种不同能力。

### 4.7 长稳、负载与采样设计

客户端允许 1..1000 个请求。单次 1000 请求最终只产生一条 run 级平均 metric，不能形成有意义的 P50/P95。建议固定 QEMU、host CPU、构建 profile 和 rootfs，执行不少于 30 轮、每轮 100 请求，得到至少 30 条 `GIPC_STARRY_METRIC` 后再聚合。

| 场景 | 建议采样 | 记录重点 | 通过口径 |
| --- | --- | --- | --- |
| 正常基线 | 30×100 请求 | run RTT、有效 B/s、errors/timeouts | 全部请求成功，errors=0，正常基线 timeout/reconnect=0 |
| 恢复压力 | 每轮固定注入一次首次断连 | attempts、timeouts、reconnects、recovery | 最终全部成功；每轮 recovery=1；单请求≤3 attempts |
| 智能侧 CPU/内存负载 | 固定背景负载下 30×100 | 与空载 P50/P95 和吞吐对比 | 无功能失败；性能门槛依据固定基线制定 |
| 长稳 | 按固定时长/轮数循环 | panic、连接泄漏、失败趋势、最大连续失败 | 无未恢复失败；原始日志完整 |

当前代码没有固定性能 SLA，不应编造绝对 RTT/吞吐门槛。建立稳定基线后，可把“P95 不高于基线 1.20 倍、平均有效 B/s 不低于基线 0.80 倍”作为项目建议回归门槛；这不是现有脚本的强制规则，必须在报告中标注基线环境和版本。

### 4.8 指标口径与计算

| 字段 | 精确口径 |
| --- | --- |
| `requests/success` | 进程请求数 / 收到合法同序列 STATUS 的请求数 |
| client `success_rate` | `success==requests` 的 0/1，不是小数比例 |
| `errors` | response magic/version/sequence/CRC 错误、ERROR 或非 STATUS 计数；server 自身 decode error 不直接进入此字段 |
| `timeouts` | connect/send/read/payload 获取失败的 attempt 计数，不全是 deadline 到期 |
| `attempts` | 每次创建 socket 并尝试 connect 均累加；STATUS 行中的值也是进程累计值 |
| `reconnects` | 同一请求 attempt>0 且 connect 成功的次数；失败的重连和正常请求间重建连接不计入 |
| `recovery` | `success>0 && reconnects>0` 的进程级 0/1 |
| `rtt_ns` | 成功 attempt 发送前到合法 STATUS 校验后的成功请求算术平均；不含此前失败 attempt |
| `throughput_bps` | `payload_len×10^9/RTT` 的成功请求算术平均；代码未乘8，实际量纲为有效响应 payload B/s |
| aggregate `success_rate` | `Σsuccess/Σrequests`，6 位小数 |
| aggregate P50/P95 | 对每条 run 平均 RTT 排序，取 `floor((M-1)q)`；不是逐请求分位 |
| aggregate `recoveries` | `recovery=1` 的 run 数，不是恢复百分比 |

`throughput_avg_bps` 是各 run 吞吐的简单平均，不按请求数加权，也不是总 payload/总墙钟时间。聚合器虽然解析 attempts 但不输出；报告若需要 attempt 总数，应从原始 metric 另行求和。

### 4.9 自动门禁与完整验收

| 门禁 | 当前自动判据 | 必须补充的完整验收 |
| --- | --- | --- |
| QEMU regex | 180 秒内 `RTOS_READY → STATUS → METRIC`；fail 为 panic/RTOS_ERROR/STARRY_TIMEOUT | 额外要求 RTOS_LISTEN、NET_READY；检查 STARRY_ERROR/NET_ERROR/RECOVERABLE_ERROR |
| `verify_metrics.py` | STATUS/METRIC 存在；无 TIMEOUT/RTOS_ERROR/STARRY_ERROR；首个 metric 的 requests==success、RTT/吞吐>0 | 多 run 必须再跑 aggregate；核对 errors 和 marker 顺序 |
| `aggregate_metrics.py` | 找到 metric；总 success=requests 且 errors=0 返回0；允许已恢复 timeout/reconnect | 检查样本数、环境一致性、sequence、原始日志和建议性能门槛 |

正常基线的强通过条件为：LISTEN、NET_READY、至少一条同序列 STATUS 和 METRIC；`requests=success`、`errors=0`、`timeouts=0`、`attempts=requests`、`reconnects=0`、`recovery=0`、RTT/吞吐>0；无 panic、RTOS_ERROR、RECOVERABLE_ERROR、NET_ERROR、STARRY_ERROR、STARRY_TIMEOUT。故障恢复场景允许预期 timeout/recoverable marker，但必须最终 `success=requests`、`errors=0`、`reconnects≥1`、`recovery=1` 且退出0。

### 4.10 实测记录与证据归档

| 用例/场景 | 平台与提交 | 请求/成功 | app errors | 传输失败 | attempts | reconnects/recovery | RTT/P50/P95 | 有效 B/s | 日志/SHA-256 | 结论 |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- | ---: | --- | --- |
| E01 网络与 listener | 待填写 | — | — | — | — | — | — | — | 待填写 | 待填写 |
| E02 单请求闭环 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 |
| E03 100 请求序列 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 |
| F03 首次断连恢复 | host mock / 待填写 commit | 1/1 | 0 | 1 | 2 | 1/1 | 从日志归档 | 从日志归档 | 待填写 | 已具备确定性入口 |
| 30×100 正常基线 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 |

每份交付记录至少包含：用例 ID、commit、QEMU/host/工具链版本、rootfs SHA-256、完整命令、输入或故障点、期望与实测结果、退出码、原始日志路径及 SHA-256、执行时间和复测人。不得用 host mock 的 RTT 作为双 guest 性能数据，也不得填写无法从归档日志复核的固定 P50/P95。

## 5. 任务三：AI 联动应用测试

任务三测试用于观察 AI 推理、控制动作生成、通信下发、控制侧执行和状态回传的完整闭环。

| 测试维度 | 关注内容 | 记录材料 |
| --- | --- | --- |
| AI 推理 | 模型加载、样例输入、推理结果、推理耗时 | 推理日志、模型配置、性能记录 |
| 动作映射 | 推理类别、置信度、控制目标、候选动作 | 映射规则、输入输出样例 |
| 控制执行 | 控制侧解析、动作执行、状态机变化；双轮足场景记录 `forward/back/left/right/stop` 到速度和偏航目标的映射 | 控制侧日志、状态回传、演示视频 |
| 闭环时延 | 输入、推理、发送、接收、执行、回传各阶段时间戳 | 端到端时延表、统计数据 |
| 异常降级 | 低置信度、通信异常、控制侧错误等场景 | 异常日志、错误码、降级状态 |

### 5.1 端到端时间戳

| 字段 | 说明 | 来源 |
| --- | --- | --- |
| `t_input` | 输入数据进入智能侧时间 | 智能侧 |
| `t_infer_start` | AI 推理开始时间 | 智能侧 |
| `t_infer_end` | AI 推理结束时间 | 智能侧 |
| `t_send` | 控制消息发送时间 | 智能侧 |
| `t_recv_control` | 控制侧收到消息时间 | 控制侧 |
| `t_action_done` | 控制动作执行完成时间 | 控制侧 |
| `t_resp_recv` | 智能侧收到状态回传时间 | 智能侧 |

### 5.2 实测记录

| 场景 | 平台 | 命令或用例 | 关键结果 | 日志位置 | 备注 |
| --- | --- | --- | --- | --- | --- |
| 模型加载与样例推理 | OrangePi 5 Plus（axvisor + starry guest，vCPU 绑定 A76 大核，板级日志 Error 档） | SenseVoice 语音识别（RK3588 NPU，librknnrt + fp16-scaled 模型），zh/en 参考 wav 各一条 | zh.wav 转写通过（fp16 精度边缘：`开放时早九点至下午五点`，参考文本 `开饭时间早上九点至下午五点`）；单条推理 1.46s、模型加载 30.62s（动态调频组＝交付配置，PR #2165 修复 governor 拓扑归因后实测：SD 冷读 29.82s，16.4 MB/s；rknn_init 0.50s）；同构建静态 1200 MHz 组推理 1.72s、加载 26.30s（冷读 25.41s，19.2 MB/s，HighSpeed 总线 78%），两组对比见 §5.3；NPU submit 7.78 ms/次，达原生 Linux 水平（约 7.5 ms） | 串口 `[perf]` 分项计时（read model / rknn_init / 推理秒数）与内核 `[perf] rknpu <kind> ioctl stat` 聚合 | 任务三；提交默认日志档为 Warn（保留 `[perf]` 聚合），上表数字取 Error 档实测 |
| 推理结果到控制动作 | 待补 | 待补 | 待补 | 待补 | 任务三 |
| 状态回传闭环 | 待补 | 待补 | 待补 | 待补 | 任务三 |
| 负载下端到端闭环 | 待补 | 待补 | 待补 | 待补 | 任务三 |
| 语音控制双轮足机器人 | Orange Pi 5 Plus + 双轮足机器人 | `assets/control_voice.wav` + StarryOS SenseVoice/RKNN + RT wheel task | 语音命令转换为有限动作集合，机器人完成对应运动并在超时后可停止 | `assets/video.mp4`、`assets/control_voice.wav`、`assets/minicom_output.jpg` | 任务三 |

### 5.3 原始数据与对比图

三配置板级实测的串口原始输出（同一 zh.wav，日志 Error 档）：

```text
# 原生 Linux（Rockchip rknpu 驱动，页缓存热读）
[perf] model load: 0.65s
{"wav": "/opt/sensevoice/testwavs/zh.wav", "text": "开放时早九点至下午五点", "seconds": 1.0377981662750244}

# starry-no-op（优化前：A55 小核、Info 日志）
[perf] model load: 41.23s
{"wav": "/opt/sensevoice/testwavs/zh.wav", "text": "开放时早九点至下午五点", "seconds": 2.9593342909999905}

# starry-opt-nolog-dynamic-v3（优化后：vCPU 绑定 A76 大核 + Error 日志 + governor 拓扑归因修复，动态调频）
[perf] read model: 29.82s (490649722 bytes)
[perf] rknn_init: 0.50s
[perf] model load: 30.62s
{"wav": "/opt/sensevoice/testwavs/zh.wav", "text": "开放时早九点至下午五点", "seconds": 1.464541749999995}
```

上面两组均为 Error 档、同一优化构建，差别只在调频策略：`starry-opt-nolog-dynamic-v3`
叠加了 PR #2165 的 governor 拓扑归因修复（动态调频）。同构建在静态 1200 MHz
（调频不介入）下为 `read model: 25.41s`、`rknn_init: 0.66s`、`model load: 26.30s`、
推理 `1.7202s`。动态组推理更快（1.46s vs 1.72s，推理期大核可升频），代价是突发
I/O 间隙会把大核降档、SD 冷读略慢（29.82s vs 25.41s）。交付配置取动态组，
静态组数据用于说明调频策略的取舍。
三配置转写文本一致（fp16 精度边缘，均输出 `开放时早九点至下午五点`，
参考文本 `开饭时间早上九点至下午五点`）。

![SenseVoice 推理与模型加载性能对比](assets/sensevoice-perf.svg)

## 6. 综合指标记录

| 指标 | 对应任务 | 平台 | 平均值 | P95 | P99 | 最大值 | 日志位置 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 控制侧周期任务延迟 | 任务一 | 待补 | 待补 | 待补 | 待补 | 待补 | 待补 |
| 客户机通信 RTT | 任务二 | 待补 | 待补 | 待补 | 待补 | 待补 | 待补 |
| AI 推理耗时 | 任务三 | OrangePi 5 Plus guest（A76 大核，日志 Error 档） | 1.46s/条（约 5.6s 音频，动态调频） | 待补 | 待补 | 8.76s（优化前 A55 小核 + Info 日志） | 串口 `[perf]` 与 `[perf] rknpu submit ioctl stat` |
| AI 到控制闭环总耗时 | 任务三 | 待补 | 待补 | 待补 | 待补 | 待补 | 待补 |
| 双轮足控制周期 | 任务一/任务三 | Orange Pi 5 Plus | 8ms 目标周期 | 待补 | 待补 | 待补 | 串口日志 / `assets/minicom_output.jpg` |
