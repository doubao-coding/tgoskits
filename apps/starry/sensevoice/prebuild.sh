#!/usr/bin/env bash
set -euo pipefail

app_dir="${STARRY_APP_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
overlay_dir="${STARRY_OVERLAY_DIR:-}"
workspace="${STARRY_WORKSPACE:-$(cd "$app_dir/../../.." && pwd)}"
arch="${STARRY_ARCH:-aarch64}"

if [[ -z "$overlay_dir" ]]; then
    echo "ERROR: STARRY_OVERLAY_DIR is required" >&2
    exit 1
fi
if [[ "$arch" != "aarch64" ]]; then
    echo "ERROR: sensevoice app only supports aarch64, got $arch" >&2
    exit 1
fi

# Download sources; SENSEVOICE_DOWNLOAD_PREFIX can point at a mirror that
# proxies GitHub (e.g. https://ghfast.top) for constrained networks.
dl_prefix="${SENSEVOICE_DOWNLOAD_PREFIX:-}"
sherpa_url_base="https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.5"
sherpa_asset="sherpa-onnx-v1.13.5-linux-aarch64-static.tar.bz2"
model_repo="csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17"
model_base="https://huggingface.co/${model_repo}/resolve/main"

cache_dir="${SENSEVOICE_CACHE_DIR:-$workspace/target/sensevoice-cache}"
mkdir -p "$cache_dir/test_wavs"

fetch() {
    local path="$1" url="$2"
    if [[ ! -s "$cache_dir/$path" ]]; then
        echo "downloading $path"
        curl -L --retry 3 -C - -o "$cache_dir/$path" "${dl_prefix}${url}"
    fi
}

fetch "$sherpa_asset" "$sherpa_url_base/$sherpa_asset"
fetch model.int8.onnx "$model_base/model.int8.onnx"
fetch tokens.txt "$model_base/tokens.txt"
fetch test_wavs/zh.wav "$model_base/test_wavs/zh.wav"
fetch test_wavs/en.wav "$model_base/test_wavs/en.wav"
fetch LICENSE "$model_base/LICENSE"

if [[ ! -x "$cache_dir/sherpa-onnx-v1.13.5-linux-aarch64-static/bin/sherpa-onnx-offline" ]]; then
    tar -xjf "$cache_dir/$sherpa_asset" -C "$cache_dir"
fi
bin_path="$cache_dir/sherpa-onnx-v1.13.5-linux-aarch64-static/bin/sherpa-onnx-offline"

# glibc arm64 runtime for the dynamic binary. Searched in the Debian cross
# sysroot first, then in a user-space extraction root.
glibc_lib_dir="${SENSEVOICE_GLIBC_ARM64_LIB_DIR:-}"
if [[ -z "$glibc_lib_dir" ]]; then
    for candidate in \
        /usr/aarch64-linux-gnu/lib \
        /workspace/tools/glibc-arm64/root/usr/aarch64-linux-gnu/lib; do
        if [[ -f "$candidate/libc.so.6" ]]; then
            glibc_lib_dir="$candidate"
            break
        fi
    done
fi
if [[ -z "$glibc_lib_dir" || ! -f "$glibc_lib_dir/ld-linux-aarch64.so.1" ]]; then
    echo "ERROR: glibc arm64 libraries not found (install libc6-dev-arm64-cross or set SENSEVOICE_GLIBC_ARM64_LIB_DIR)" >&2
    exit 1
fi

# Integrity gate: every installed artifact must match SHA256SUMS.
sha256sums="$app_dir/SHA256SUMS"
[[ -f "$sha256sums" ]] || { echo "ERROR: missing $sha256sums" >&2; exit 1; }
check() {
    local path="$1" expected="$2" actual
    actual=$(sha256sum "$path" | awk '{print $1}')
    if [[ "$actual" != "$expected" ]]; then
        echo "ERROR: sha256 mismatch for $path" >&2
        echo "  expected $expected" >&2
        echo "  actual   $actual" >&2
        exit 1
    fi
}
expected_hash() {
    awk -v key="$1" '$2 == key {print $1}' "$sha256sums"
}
check "$bin_path" "$(expected_hash sherpa-onnx-offline)"
for f in model.int8.onnx tokens.txt test_wavs/zh.wav test_wavs/en.wav; do
    check "$cache_dir/$f" "$(expected_hash "$f")"
done
for f in ld-linux-aarch64.so.1 libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2; do
    check "$glibc_lib_dir/$f" "$(expected_hash "$f")"
done

install -Dm0755 "$bin_path" "$overlay_dir/opt/sensevoice/bin/sherpa-onnx-offline"
install -Dm0644 "$cache_dir/model.int8.onnx" "$overlay_dir/opt/sensevoice/model/model.int8.onnx"
install -Dm0644 "$cache_dir/tokens.txt" "$overlay_dir/opt/sensevoice/model/tokens.txt"
install -Dm0644 "$cache_dir/test_wavs/zh.wav" "$overlay_dir/opt/sensevoice/model/zh.wav"
install -Dm0644 "$cache_dir/test_wavs/en.wav" "$overlay_dir/opt/sensevoice/model/en.wav"
install -Dm0644 "$cache_dir/LICENSE" "$overlay_dir/opt/sensevoice/model/LICENSE"
install -Dm0755 "$app_dir/sensevoice-test.sh" "$overlay_dir/usr/bin/sensevoice-test.sh"

# glibc loader on its PT_INTERP path plus the NEEDED libraries under /lib.
install -Dm0755 "$glibc_lib_dir/ld-linux-aarch64.so.1" "$overlay_dir/lib/ld-linux-aarch64.so.1"
install -Dm0755 "$glibc_lib_dir/libc.so.6" "$overlay_dir/lib/libc.so.6"
install -Dm0755 "$glibc_lib_dir/libm.so.6" "$overlay_dir/lib/libm.so.6"
install -Dm0755 "$glibc_lib_dir/libpthread.so.0" "$overlay_dir/lib/libpthread.so.0"
install -Dm0755 "$glibc_lib_dir/libdl.so.2" "$overlay_dir/lib/libdl.so.2"
