# Board bring-up: OrangePi 5 Plus SenseVoice NPU

烧录与运行步骤（板子到手后按序执行；所有材料已离线备好）。

## 0. 前置条件

- OrangePi 5 Plus（RK3588，8G/16G 均可），Type-C 串口线，网线或 USB 网卡
- 宿主机：本仓库 + `assets/`（含 sensevoice、sensevoice-rknn）+ `/workspace/tools` 工具链
  （复现包 `assets/m3-repro-pack/setup-host.sh` 可一键装好）
- 板卡 rootfs：沿用仓库 OrangePi-5-Plus 板级流程（starry 板用例的标准镜像）

## 1. 先在板子出厂 Linux 上验证 NPU 链路（强烈建议的第一步）

这一步不碰 StarryOS，只确认 NPU 硬件/驱动/模型三件套正常，排除硬件问题：

```bash
cargo xtask board connect -b OrangePi-5-Plus     # 持锁，等 Linux 起来，记下 IP

# 另开终端，推资产到板子
ssh orangepi@<IP> mkdir -p /tmp/svrknn
scp assets/sensevoice-rknn/{librknnrt.so,am.mvn,embedding.npy,chn_jpn_yue_eng_ko_spectok.bpe.model} orangepi@<IP>:/tmp/svrknn/
scp assets/sensevoice-rknn/sense-voice-encoder.rk3588.fp16-scaled.rknn orangepi@<IP>:/tmp/svrknn/

ssh orangepi@<IP>
sudo cp /tmp/svrknn/librknnrt.so /usr/lib/
dmesg | grep -i rknpu          # 期待看到 rknpu 驱动 probe 成功（fdab0000.npu）
ls /dev/dri/                   # 期待 card1 存在
```

出厂系统若带 python3+numpy 可直接跑 `sensevoice_rknn_npu.py --model-dir
/tmp/svrknn --lib-dir /tmp/svrknn --wav zh.wav` 做首次 NPU 推理冒烟。

## 2. StarryOS 侧装配与运行

```bash
. /workspace/tools/tgos-env.sh    # 宿主机工具环境（含 qemu-user-static 需另装）
cd /workspace/tgoskits

# 需先装 qemu-user-static（prebuild 用它跑目标架构 apk）：
#   sudo apt install qemu-user-static   （或用用户空间解包同前述模式）

cargo xtask starry app board -t sensevoice-rknn -b OrangePi-5-Plus
```

判定：串口出现 `SENSEVOICE_RKNN_TEST_PASSED`（L0–L3 全过）。

## 3. 可能需要现场调的两处（已知风险点）

| 风险 | 症状 | 处置 |
|---|---|---|
| BPE 解析器过于保守 | L2/L3 transcript 为空但 logits 有值 | `load_bpe_pieces` 换用正式 sentencepiece（glibc wheel + cpython3.10 不可行时，改用纯 py 的 tokenizer 实现；tokens 顺序与 sherpa 的 tokens.txt 一致，可从 `assets/sensevoice/tokens.txt` 直接按行读） |
| LFR 帧数对不齐 | 输出乱码/长度异常 | RKNN 模型是 fixed-shape（scaled.fixed），推理前需 pad/裁剪到模型固定 T；`rknn_query` 的 `in_attr.n_dims` 会给出，`transcribe()` 里按它对齐 |
| NPU 驱动版本不匹配 | rknn_init 返回非 0 | 板上内核 rknpu 驱动过旧时，用 `assets/sensevoice-rknn/librknnrt.so`（2.3.2，与内核 0.9.x 驱动兼容）；仍失败则查 dmesg |

## 4. 性能预期

- happyme531 实测口径：单 NPU 核 RTF ≈ 0.05（20×实时），内存 ~1.1G
- 对照（本项目实测）：StarryOS CPU（QEMU 模拟）RTF 3.14 → 真板 NPU 预期 ~60× 加速
- 跑分输出：`[perf] model load: X.XXs` 行 + 每 wav 一行 JSON

## 5. 资产位置速查

| 内容 | 路径 |
|---|---|
| NPU 模型 + 解码资产 + librknnrt | `assets/sensevoice-rknn/`（SHA256SUMS.assets） |
| CPU 版模型/二进制（对照） | `assets/sensevoice/` |
| 推理脚本 | `apps/starry/sensevoice-rknn/sensevoice_rknn_npu.py` |
| 测试脚本 | `apps/starry/sensevoice-rknn/sensevoice-rknn-test.sh` |
| QEMU 全链路复现包 | `assets/m3-repro-pack/` |
