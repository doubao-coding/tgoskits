# sensevoice-rknn — OrangePi 5 Plus (RK3588) SenseVoice NPU 板级用例

在 StarryOS 上用 RK3588 NPU 运行 SenseVoiceSmall 语音识别（zh/en/ja/ko/yue）。
CPU 只做 fbank 特征与 CTC 解码，encoder 跑在 NPU 上。

## 需要板子的原因

RKNN 依赖 RK3588 NPU 硬件（`/dev/dri/card1`，内核 rknpu 驱动 + 用户态
librknnrt.so），QEMU 无该设备模型。本目录所有内容都已离线备好，拿到板子后
按 BOARD-RUNBOOK.md 烧录执行即可。

## 架构

```
wav (16k s16 mono)
  └─ CPU: fbank80 (自实现 kaldi 兼容前端, 纯 numpy) + CMVN(am.mvn) + LFR(7,6)
  └─ NPU: sense-voice-encoder.rk3588.fp16-scaled.rknn (librknnrt C API via ctypes)
  └─ CPU: CTC greedy + sentencepiece BPE (自实现, 读 .bpe.model)
  └─ 文本
```

Python 3.12 (musl, Alpine v3.23 apk) + py3-numpy。librknnrt.so 及其 glibc
依赖（libc.so.6/libm/libpthread/libdl/libstdc++/libgcc_s）随 overlay 提供，
通过 glibc 动态加载器运行（仓库 glibc-dynamic-smoke 已验证该模式）。
绑定不使用 rknnlite Python 包（它是 cp310 glibc wheel，与 musl CPython 不
兼容）；直接 ctypes 调 C API，`.so` 只依赖 GLIBC_2.17。

## 资产（全部在 assets/sensevoice-rknn/，SHA256SUMS.assets 钉死）

| 文件 | 说明 |
|---|---|
| sense-voice-encoder.rk3588.fp16-scaled.rknn (490M) | NPU 模型（fp16 + 激活缩放防溢出），harvestsu/sensevoice-rknn |
| sense-voice-encoder.scaled.fixed.onnx (937M) | 同源 ONNX（板端 CPU 对照/debug 用） |
| am.mvn / embedding.npy / chn_jpn_yue_eng_ko_spectok.bpe.model | 特征归一化 / query 向量 / BPE tokenizer |
| librknnrt.so (2.3.2) | RKNN 用户态运行库 |
| rknn_toolkit_lite2-*.whl | （备用）Python 绑定，glibc CPython 3.10 用 |

模型许可证：FunASR Model Open Source License（署名再分发；归属链见
资产目录 README.md）。本用例不包含任何 AGPL 运行时代码，推理脚本为独立实现。

## 运行（板子到手后）

见 BOARD-RUNBOOK.md；一句话版：

```bash
cargo xtask starry app board -t sensevoice-rknn -b OrangePi-5-Plus
```
