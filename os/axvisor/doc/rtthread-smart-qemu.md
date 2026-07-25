# RTT-Smart AArch64 QEMU Guest

This document describes how to build RTT-Smart and boot it as an AxVisor guest on AArch64 QEMU.

## Prerequisites

Install the following host tools:

- Bash
- SCons
- QEMU with `qemu-system-aarch64`
- Device Tree Compiler tools with `fdtget` and `fdtput`
- `curl` or `wget`
- `tar` with bzip2 support

The RTT-Smart build script downloads the official AArch64 musl toolchain into
`${XDG_CACHE_HOME:-$HOME/.cache}/rt-thread/toolchains` when it is not already available.

## Build RTT-Smart

Clone the AxVisor guest branch of RT-Thread and run its image build script:

```bash
git clone --branch axvisor-rtt-smart \
  https://github.com/doubao-coding/rt-thread.git
cd rt-thread
./bsp/qemu-virt64-aarch64/build-axvisor-images.sh "$PWD/output/axvisor"
```

The output directory contains:

```text
rtthread-smart-aarch64.bin
rtthread-smart-aarch64.elf
rtthread-smart-aarch64.dtb
```

The script builds a single-vCPU RTT-Smart kernel for RAM at `0x8000_0000`. It also generates a
GICv3 QEMU DTB and configures `/chosen` to use the shared PL011 at `0x0900_0000` as its early
console.

## Configure AxVisor

From the tgoskits workspace root, copy the VM template to the ignored `tmp` directory:

```bash
mkdir -p tmp
cp os/axvisor/configs/vms/qemu/aarch64/rtthread-smart-smp1.toml \
  tmp/rtthread-smart-smp1.toml
```

Edit these two fields in `tmp/rtthread-smart-smp1.toml` to use the absolute paths generated in the
previous step:

```toml
kernel_path = "/absolute/path/to/rt-thread/output/axvisor/rtthread-smart-aarch64.bin"
dtb_path = "/absolute/path/to/rt-thread/output/axvisor/rtthread-smart-aarch64.dtb"
```

Do not change the configured load addresses. RTT-Smart enters at `0x8008_0000`, receives the DTB
address in `x0`, and builds its early page tables for RAM starting at `0x8000_0000`. The VM RAM must
therefore remain `MAP_ALLOC` at that guest physical address.

## Run AxVisor

Run from the tgoskits workspace root:

```bash
cargo xtask axvisor qemu \
  --arch aarch64 \
  --smp 4 \
  --vmconfigs tmp/rtthread-smart-smp1.toml
```

AxVisor and RTT-Smart share QEMU's PL011, so their output appears in the same terminal. AxVisor
debug logging can interleave with guest output; use the default information log level for a more
readable console.

## Expected Output

A successful boot includes the early console selection and RTT-Smart banner:

```text
[I/rtdm.ofw] Console: uart0 (pl011@9000000)

 \ | /
- RT -     Thread Smart Operating System
 / | \     5.3.0 build ...
 2006 - 2024 Copyright by RT-Thread team
```

Exit QEMU with `Ctrl-A`, then `X`.

## Troubleshooting

- If the vCPU entry is not `0x8008_0000`, confirm that the VM RAM mapping type is `MAP_ALLOC` (`0`),
  not `MAP_IDENTICAL` (`1`).
- If RTT-Smart runs without console output, inspect the generated DTB and confirm that
  `/chosen/bootargs` contains `earlycon` and `/chosen/stdout-path` is `/pl011@9000000`.
- If the build uses `aarch64-none-elf-gcc`, remove that override. RTT-Smart requires the
  `aarch64-linux-musleabi` toolchain selected by the build script.
