# 任务二测试证据摘录（2026-08-23 UTC）

## 1. 固定环境

```text
implementation commit: 2a891bbe949f59c7bfd037107b3615de4f57c704
host: Linux e4bb9163e4b0 4.19.90-89.11.v2401.ky10.x86_64 x86_64
QEMU: 10.0.11 (Debian 1:10.0.11+ds-0+deb13u1)
rustc: 1.99.0-nightly (da80ed070 2026-07-14)
cargo: 1.99.0-nightly (59800466c 2026-07-07)
cc: Debian 14.2.0-19
Python: 3.13.5
```

## 2. 自动化结果

```text
cargo test -p guest-ip-protocol
test result: ok. 5 passed; 0 failed; 0 ignored

cargo test -p axvirtio-net
unit tests: 14 passed; integration tests: 18 passed; total: 32 passed

cargo test -p axvirtio-blk
unit tests: 3 passed; integration tests: 34 passed; doctests: 1 passed; total: 38 passed

cargo clippy -p guest-ip-protocol --all-targets -- -D warnings
exit: 0

cargo check -p arceos-guest-ip-server --no-default-features
exit: 0

cargo clippy -p arceos-guest-ip-server --no-default-features --all-targets -- -D warnings
exit: 0

cc -std=c11 -Wall -Wextra -Werror -O2 apps/starry/guest-ip-link/linux-client.c ...
exit: 0

python3 -m py_compile scripts/test/guest-ip-link/*.py
exit: 0
```

## 3. 首次断连恢复

```text
GIPC_STARRY_STATUS seq=1 payload=8 attempts=2 timeouts=1
GIPC_STARRY_METRIC requests=1 success=1 success_rate=1 errors=0 timeouts=1 attempts=2 reconnects=1 recovery=1 rtt_ns=84449 throughput_bps=94731
GIPC_METRICS_OK
GIPC_AGGREGATE requests=1 success=1 success_rate=1.000000 app_errors=0 timeouts=1 reconnects=1 recoveries=1 rtt_p50_ns=84449 rtt_p95_ns=84449 throughput_avg_bps=94731
```

恢复测试原始日志 SHA-256：

```text
a9ee4f39a51b75051f7dd12522ab8bcd49299e0afe82f1ece1153ec4dd819137
```

## 4. 双 guest QEMU 复测

构建与装配阶段成功生成以下 AArch64 产物：

```text
StarryOS kernel BIN  1ab3c90f33ac178e3fda1c6992ca76b164b4d7c2c91d22c52c97d5165cede633
ArceOS server ELF   ba5da5abf48a32199c8e4fd7b64c4e9026573ccbbfa545fede23c0842dbfab64
StarryOS client ELF e4b16fdce3c7821df320dcf77804a56f576676a3171d48bd48747d6a56d53922
rootfs before run   b2e31e1c45d54a2a6b08f8830f1cd85868e3cf02fe647c2dc4673b4f98c2fd4e
```

QEMU 实际 marker：

```text
VM[1] boot success
VM[2] boot success
[VM 1] panic
[VM 1] failed to determine root device from available block devices
=== FAIL PATTERN MATCHED: (?i)panic
guest_e2e status=1 elapsed_ms=20007
```

本轮完整 QEMU 日志 SHA-256：

```text
f8598c33909a424b30de00547dad32d6b12c4fa5fe426a0390af0664faf2c3a2
```

本轮能证明 Axvisor、VM 创建、vCPU 启动和两个 guest 内核入口均已运行；StarryOS 在网络初始化前因缺少可用 root block device 停机，因此不能从该日志导出 LISTEN、NET_READY、STATUS、METRIC 或双 guest RTT/吞吐量。
