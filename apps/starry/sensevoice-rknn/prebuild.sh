#!/usr/bin/env bash
# sensevoice-rknn 板级 app 的资产装配（在宿主机上运行，产出 staging 目录）。
# 与 py-sci 的 prebuild 模式一致：qemu-user-static + Alpine apk 装目标架构闭包。
#
# 输入：
#   ASSETS_DIR       默认 <workspace>/assets/sensevoice-rknn（模型/库/whl）
#   STAGING_ROOT     框架提供的 staging 树（含 base rootfs）
# 输出（overlay 布局，直接注入板卡 rootfs 镜像）：
#   /opt/sensevoice/{bin,python,lib,glibc,model,testwavs}
#   /usr/bin/sensevoice-rknn-test.sh
set -euo pipefail

app_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace="${STARRY_WORKSPACE:-$(cd "$app_dir/../../.." && pwd)}"
assets="${SENSEVOICE_ASSETS_DIR:-$workspace/assets/sensevoice-rknn}"
staging="${STARRY_STAGING_ROOT:?need staging root}"
arch="${STARRY_ARCH:-aarch64}"
out="${STARRY_OVERLAY_DIR:?need overlay dir}"

case "$arch" in
    aarch64) qemu_runner="qemu-aarch64-static" ;;
    *) echo "ERROR: sensevoice-rknn only supports aarch64, got $arch" >&2; exit 1 ;;
esac
command -v "$qemu_runner" >/dev/null || { echo "ERROR: need qemu-user-static ($qemu_runner)" >&2; exit 1; }

# ---- 1. Python 3.12 + numpy (musl) via Alpine apk into staging ----
branch="v3.23"
repo="https://dl-cdn.alpinelinux.org/alpine"
printf '%s/%s/main\n%s/%s/community\n' "$repo" "$branch" "$repo" "$branch" \
    > "$staging/etc/apk/repositories"
"$qemu_runner" /usr/sbin/apk add --root "$staging" --no-scripts \
    python3 py3-numpy >/dev/null

py3="$(ls "$staging"/usr/lib | grep -E '^python3\.[0-9]+$' | sort -V | tail -1)"

# ---- 2. glibc 运行库（librknnrt 及其 NEEDED；从 Debian arm64 cross 提取） ----
glibc_src="${SENSEVOICE_GLIBC_ARM64_LIB_DIR:-}"
if [[ -z "$glibc_src" ]]; then
    for candidate in /usr/aarch64-linux-gnu/lib /workspace/tools/glibc-arm64/root/usr/aarch64-linux-gnu/lib; do
        [[ -f "$candidate/libc.so.6" ]] && glibc_src="$candidate" && break
    done
fi
[[ -n "$glibc_src" && -f "$glibc_src/ld-linux-aarch64.so.1" ]] || {
    echo "ERROR: glibc arm64 runtime not found (set SENSEVOICE_GLIBC_ARM64_LIB_DIR)" >&2
    exit 1
}

# ---- 3. 落盘 overlay ----
sv="$out/opt/sensevoice"
mkdir -p "$sv/bin" "$sv/python" "$sv/lib" "$sv/glibc" "$sv/model" "$sv/testwavs" \
         "$out/usr/bin"

cp "$staging/usr/bin/python3.$(echo "$py3" | cut -d. -f1-2)" "$sv/bin/python3" 2>/dev/null || \
    cp "$staging/usr/bin/python3" "$sv/bin/python3"

# musl python 需要标准库与 numpy 的 site-packages；整树拷贝保持 import 可用
cp -r "$staging/usr/lib/$py3" "$sv/python/$py3"
mkdir -p "$sv/python/site-packages"
for sp in "$staging/usr/lib/$py3/site-packages"/*; do
    [[ -e "$sp" ]] && cp -r "$sp" "$sv/python/site-packages/"
done
# numpy 及其共享库闭包在 /usr/lib 下（libopenblas 等）
mkdir -p "$sv/python/extra-lib"
find "$staging/usr/lib" -maxdepth 1 -name "*.so*" -newer "$staging/etc" -exec cp {} "$sv/python/extra-lib/" \; 2>/dev/null || true

install -m0755 "$assets/librknnrt.so" "$sv/lib/librknnrt.so"
install -m0755 "$app_dir/sensevoice_rknn_npu.py" "$sv/python/sensevoice_rknn_npu.py"
install -m0755 "$app_dir/sensevoice-rknn-test.sh" "$out/usr/bin/sensevoice-rknn-test.sh"

for f in ld-linux-aarch64.so.1 libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 \
         libstdc++.so.6 libgcc_s.so.1; do
    install -m0755 "$glibc_src/$f" "$sv/glibc/$f"
done

install -m0644 \
    "$assets/sense-voice-encoder.rk3588.fp16-scaled.rknn" \
    "$assets/am.mvn" "$assets/embedding.npy" "$sv/model/"
install -m0644 "$workspace/assets/sensevoice/tokens.txt" "$sv/model/tokens.txt"
install -m0644 "$workspace/assets/sensevoice/test_wavs/zh.wav" "$sv/testwavs/zh.wav"
install -m0644 "$workspace/assets/sensevoice/test_wavs/en.wav" "$sv/testwavs/en.wav"

echo "sensevoice-rknn overlay assembled at $out"
