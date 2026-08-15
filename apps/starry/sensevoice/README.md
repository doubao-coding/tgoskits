# SenseVoice ASR on StarryOS (QEMU, CPU inference)

Runs the SenseVoice speech-recognition model (FunAudioLLM SenseVoiceSmall,
int8 ONNX export, zh/en/ja/ko/yue) fully inside a StarryOS QEMU guest using
the `sherpa-onnx-offline` CLI, on emulated CPU only. This case is the
operator-facing workload behind the "AI workload on Starry" effort and is
kept independent of the Axvisor guest work.

## Test command

```bash
cargo xtask starry app run -t sensevoice --arch aarch64
```

## What it exercises

| Level | Check |
|-------|-------|
| L0 | `sherpa-onnx-offline --help` executes (glibc dynamic loader + libs under StarryOS) |
| L1 | missing model path fails gracefully with a diagnostic |
| L2 | full inference on `zh.wav`: transcript must contain `开饭时间早上九点至下午五点` |
| L3 | full inference on `en.wav`: transcript must contain `the tribal chieftain` |

Both reference transcripts were captured with the same binary + model on a
native x86_64 host; the guest assertion only checks stable substrings so
integer-path nondeterminism across platforms cannot flake the test.

## Assets

`prebuild.sh` installs from the download cache (`target/sensevoice-cache`,
resumable via `SENSEVOICE_DOWNLOAD_PREFIX` mirror override):

- `sherpa-onnx-offline` v1.13.5 aarch64 build (glibc dynamic; C++ runtime and
  onnxruntime are statically linked into the binary)
- glibc arm64 loader + `libc.so.6` `libm.so.6` `libpthread.so.0` `libdl.so.2`
- SenseVoice int8 ONNX model (239 MiB), `tokens.txt`, two reference wavs

Integrity is pinned by `SHA256SUMS` next to this README. The dedicated
rootfs image `rootfs-aarch64-sensevoice.img` (a resized copy of the managed
Alpine image) receives the assets through the standard overlay injection, so
the shared Alpine image is not polluted:

```bash
cargo xtask image resize tmp/axbuild/rootfs/rootfs-aarch64-sensevoice.img/rootfs-aarch64-sensevoice.img --size-mib 4096
```

## Measured performance

QEMU TCG (`cortex-a53`, 1 vCPU, `-m 4g`), 2026-08-15, sherpa-onnx v1.13.5,
int8 model, `--num-threads=1`:

| Clip | Audio | Inference | RTF |
|------|-------|-----------|-----|
| zh.wav | 5.59 s | 13.96 s | 2.50 |
| en.wav | 6.30 s | (same run, total QEMU time incl. boot + all levels: 70 s) | |

The case timeout is 1800 s to leave headroom for slower hosts.
`sys_prctl: unsupported option 51/64` lines in the boot log are glibc
feature probes and are harmless.
