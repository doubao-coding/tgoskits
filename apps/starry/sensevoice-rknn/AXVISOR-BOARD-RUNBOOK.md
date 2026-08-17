# Axvisor + StarryOS + SenseVoice RKNN on OrangePi 5 Plus — 交接 runbook

在 OrangePi 5 Plus（RK3588）上用 **axvisor（Type-1 hypervisor）启动 starry guest**，在
guest 里经 **RK3588 NPU** 跑 SenseVoiceSmall 语音识别（librknnrt + .rknn 模型）。
本 runbook 记录把整条链路跑通的过程、踩到的 starry rknpu 驱动 gap 及绕法、以及剩余
的已知问题与下一步。

## 0. 链路总览（已跑通）

```
U-Boot (ostool uboot runner, FIT bootm, MMU-off + fdt in x0)
└─ axvisor (EL2, axplat-dyn, rockchip-sdhci+fs+dwmmc)
   └─ starry guest (passthrough, 1 GiB low mem, /dev/dri/card1 = rknpu)
      └─ librknnrt 2.3.2 (glibc 动态, 板自带 python3.10 + numpy)
         rknn_init → rknn_query → inputs_set → rknn_run(NPU 执行) → outputs_get
```

NPU 真的执行了模型（237 任务，`hw_elapse_time` 有值）。链路通。最后卡在 fp16 溢出
（见 §5）。

## 1. 构建

```bash
# axvisor（板级，带 starry-smp1 guest）
cargo xtask axvisor build \
  --config os/axvisor/configs/board/orangepi-5-plus.toml \
  --vmconfigs os/axvisor/configs/vms/orangepi-5-plus/starry-smp1.toml
# 产物：target/aarch64-unknown-linux-musl/release/axvisor{,.bin}（注意是 musl PIE target）

# starry guest（板级，带 rknpu/rockchip-sdhci）
cargo xtask starry build --config os/StarryOS/configs/board/orangepi-5-plus.toml
# 产物：target/aarch64-unknown-none-softfloat/release/starryos.bin
```

axvisor 引导用 FIT（`axvisor.bin` + `orangepi-5-plus.dtb` 打包，load/entry `0x20000000`）。
手敲 U-Boot 可用 `bootm 0x20000000`；裸 `axvisor.bin` 用 `booti` 需先加 ARM64 Image
header（magic `ARMd` + `b` 到 offset 0x40）。**推荐用 ostool uboot runner**（见 §3），
它自动处理 MMU-off + fdt + EL2 进入，是 CI 验证过的路径。

## 2. rootfs（板子出厂 SD 的 ext4 rootfs，方案 A：不重做镜像，只加文件）

板出厂是 OrangePi Jammy（Ubuntu 22.04，自带 python3.10；**无 numpy**）。
读卡器挂 rootfs 分区（如 `/dev/sda2`），`e2fsck -fy` 清脏 journal 后挂可写，往里加：

- **starry guest**：`starryos.bin` → `/guest/starry/starryos.bin`（axvisor 从 fs 读 guest，
  `starry-smp1.toml` 的 `kernel_path` 指这里）。
- **RKNN 资产**（`/opt/sensevoice/{lib,model,python,testwavs}` + `/usr/bin/sensevoice-rknn-test.sh`）：
  `librknnrt.so`、`sense-voice-encoder.rk3588.fp16-scaled.rknn`、`am.mvn`、`embedding.npy`、
  `tokens.txt`、`zh/en.wav`、`sensevoice_rknn_npu.py`。
- **numpy + BLAS/LAPACK/gfortran**（板子没有 numpy；从板 apt 索引解析 jammy arm64 依赖闭包，
  下 `.deb` 解包，**不用 apt/qemu-user-static**）：`python3-numpy`、`libblas3`、`liblapack3`、
  `libgfortran5`。放 `/usr/lib/python3.10/dist-packages/numpy` + `/opt/sensevoice/lib/`。
- **glibc 用板自带的**（`/lib/aarch64-linux-gnu`），不要装 overlay 的 /lib glibc（会覆盖坏板子系统）。

写完务必 `sync` + `umount` 干净卸载，否则 ext4 journal 脏 → starry 挂时 "error state, replaying
journal, read-only"，重放可能丢刚写的文件。

## 3. 引导（ostool uboot runner，推荐）

```bash
# .uboot.toml（工作区根，runner 读它）：serial=/dev/ttyUSB0 baud=1500000
#   dtb_file=os/StarryOS/configs/board/orangepi-5-plus.dtb
cargo xtask axvisor test uboot --board orangepi-5-plus --guest starry
```

runner 自动构建 axvisor + 串口复位板子 + FIT bootm 引导 + 抓 `starry guest test pass`。
手敲 U-Boot 见 §1（`bootm 0x20000000`）。**别用 `go`**（不关 MMU，axvisor 早期 boot 直接 fault，
silent）。

## 4. 跑测试

starry 到 `root@starry:/root #` 后：

```bash
MALLOC_MMAP_THRESHOLD_=999999999 \
LD_LIBRARY_PATH=/lib/aarch64-linux-gnu:/usr/lib/aarch64-linux-gnu:/opt/sensevoice/lib \
PYTHONPATH=/opt/sensevoice/python \
/opt/sensevoice/bin/python3 /opt/sensevoice/python/sensevoice_rknn_npu.py \
  --wav /opt/sensevoice/testwavs/zh.wav
```

`SENSEVOICE_DEBUG=1` 打 in/out attr、输入/输出值分布（见 §5 诊断）。`SENSEVOICE_INPUT_SCALE=X`
缩放输入（试过 0.5，对 §5 的 fp16 溢出无效，输入已经很小）。

## 5. starry rknpu 驱动的 gap 及绕法（本 runbook 的核心）

starry 的 rknpu 驱动是**重实现**，跟 librknnrt 2.3.2 预期在多个语义点对不齐。已绕过 4 个，
第 5 个（fp16 计算正确性）绕不掉。

| # | 症状 | 根因 | 绕法（已做） |
|---|---|---|---|
| 1 | `malloc.c: mremap_chunk: prev_size(p)==offset` 断言 | starry 的 mremap/mmap 跟 glibc 2.35 malloc 不兼容（大块 realloc） | `export MALLOC_MMAP_THRESHOLD_=999999999`（不走 mmap 路径） |
| 2 | `ax-fs-ng cache reclaim: sleeping in atomic context` panic | 读 490MB .rknn 涨页缓存 → reclaim 在 preempt_disabled 里睡眠 | `starry-smp1.toml` 低内存区 256MiB→1GiB |
| 3 | `rknn_query(OUTPUT_ATTR) failed: -5`（`info_len(312)<sizeof(rknn_tensor_attr)(376)`） | 脚本 `RknnTensorAttr` 缺 `dims[16]`（librknnrt 2.x 把 dims 插在 n_dims 后） | 脚本 struct 加 `("dims", c_uint32*16)` 在 n_dims 后（size 312→376） |
| 4 | `rknn_wait fence fd=-1 is invalid` | starry 驱动没实现 dma_fence（librknnrt 2.x run 异步靠 fence 同步） | 脚本把 rknn_wait 失败降级为 non-fatal（run 是同步的，ioctl 返回时输出就绪） |
| 5 | **输出全 `inf`**（`logits[:]=[inf,inf,...]`） | starry 驱动没正确复刻真 Rockchip 驱动的 **fp16 激活缩放** → 中间激活超 65504 → 全 inf | **未绕过**。输入很小（batch max≈2.06）仍 inf，说明溢出在模型内部，不是输入幅度问题 |

附带的驱动不一致（未致命，但要注意）：
- `rknn_query(INPUT_ATTR)` 返回全 0（输入是动态 shape，attr 查不出来；拿不到固定 `T_in` 做输入 pad）。
- `outputs_get` 的 `out.size` 是 bogus（返回 8618920=`344×25055`，但 out attr 说是 `[1,344,560]`=192640；44 倍差）。

## 6. 当前状态（交接点）

- ✅ axvisor EL2 启动、starry guest 起、NPU 透传（`/dev/dri/card1`）、librknnrt
  rknn_init/run/outputs 全链路在 starry guest 里跑通，NPU 真执行了模型。
- ✅ 绕过 starry 驱动 4 个 gap（见 §5 表 1–4）。
- ❌ §5 #5：**fp16 激活缩放在 starry 驱动下没生效 → 输出全 inf → CTC 解码全 blank → 空文本**。

## 7. 下一步（建议优先级）

1. **重新把模型量化成 int8 `.rknn`**（最可能直接绕过 fp16 溢出）。assets 里有
   `sense-voice-encoder.scaled.fixed.onnx`（937MB），用 rknn-toolkit2 在 x86 上转：
   `do_quantization=True, quantized_dtype=INT8`，产出 int8 `.rknn` 替换 `assets/sensevoice-rknn/`
   的 fp16 版。int8 范围 −128~127 不会溢出。
2. **在原生 Linux 上验一次**（真 Rockchip rknpu 驱动）：同样 fp16 模型 + librknnrt 跑。
   原生不 inf、starry 下 inf → 坐实是 starry 驱动 fp16 执行的 gap。
3. **修 starry rknpu 驱动的 fp16 执行**（`drivers/ax-driver` 的 rknpu 实现）：对齐真 Rockchip
   驱动的激活缩放/fp16 算子语义。工作量大，是 §5 #5 的根治。
4. 前端余项（§5 附带）：动态输入要 `rknn_set_input_shapes`；输出 reshape 按 out attr
   `[1,344,560]` 取前 `n_elems` 个（别按 bogus `out.size`）；若模型只输出 encoder hidden
   `[1,344,560]`（非 CTC logits `[344,25055]`），CPU 上要补 CTC head（vocab 投影）。

## 8. 相关文件

| 文件 | 说明 |
|---|---|
| `apps/starry/sensevoice-rknn/sensevoice_rknn_npu.py` | 推理脚本（含 §5 #3/#4 绕法 + §5 诊断打印） |
| `apps/starry/sensevoice-rknn/sensevoice-rknn-test.sh` | L0–L3 测试入口 |
| `os/axvisor/configs/vms/orangepi-5-plus/starry-smp1.toml` | starry guest 配置（低内存 1GiB；§5 #2） |
| `os/axvisor/configs/board/orangepi-5-plus.toml` | axvisor 板级配置（+dwmmc） |
| `test-suit/axvisor/normal/board-orangepi-5-plus/starry/` | ostool uboot 测试用例 |
| `apps/starry/sensevoice-rknn/BOARD-RUNBOOK.md` | 直跑 starry（非 axvisor）的板级 runbook |
