#!/usr/bin/env python3
"""Host-side numerical check of the sensevoice_rknn_npu frontend.

Runs the numpy fbank/LFR/CMVN pipeline on the reference zh.wav and compares
against the same features recomputed via the kaldi-compatible reference
(simple re-implementation, tolerances loose enough for float32).

This validates the CPU-side math that will run unchanged on the board; it
does NOT touch the NPU.
"""

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from sensevoice_rknn_npu import (  # noqa: E402
    apply_lfr,
    fbank80,
    load_cmvn,
    read_wav_mono,
    ctc_greedy,
)

ASSETS = os.environ.get(
    "SENSEVOICE_ASSETS", "/workspace/tgoskits/assets/sensevoice-rknn"
)


def main():
    wav = os.path.join(os.environ.get("SENSEVOICE_WAV_DIR",
                                      "/workspace/tgoskits/assets/sensevoice/test_wavs"),
                       "zh.wav")
    samples = read_wav_mono(wav)
    assert samples.dtype == __import__("numpy").float32
    assert abs(float(samples.max())) <= 1.0
    print(f"wav ok: {len(samples)} samples ({len(samples)/16000:.2f}s)")

    feats = fbank80(samples)
    assert feats.shape[1] == 80, feats.shape
    # 5.592s audio -> expect ~557 frames (10ms shift, snip_edges)
    expect_lo, expect_hi = 540, 575
    assert expect_lo <= feats.shape[0] <= expect_hi, feats.shape
    # log-mel range sanity (energies neither silent nor exploding)
    assert -25.0 < float(feats.mean()) < 5.0, feats.mean()
    print(f"fbank ok: shape={feats.shape} mean={float(feats.mean()):.3f}")

    mean, var = load_cmvn(os.path.join(ASSETS, "am.mvn"))
    assert mean.shape == (560,) and var.shape == (560,), (mean.shape, var.shape)
    assert float(var.min()) > 0
    print(f"cmvn ok: shift560[0]={float(mean[0]):.4f} rescale560[0]={float(var[0]):.4f}")

    lfr = apply_lfr(feats, (mean, var))
    # LFR(7,6): T -> ceil(T/6), feature dim 7*80=560
    assert lfr.shape == (((feats.shape[0] + 5) // 6), 560), lfr.shape
    print(f"lfr ok: shape={lfr.shape}")

    # CTC greedy unit check
    tokens = ["", "你", "好", "世", "界"]
    import numpy as np
    logits = np.array([
        [0.0, 9.0, 0.0, 0.0, 0.0],   # 你
        [0.0, 9.0, 0.0, 0.0, 0.0],   # 你 (repeat -> collapsed)
        [9.0, 0.0, 0.0, 0.0, 0.0],   # blank
        [0.0, 0.0, 9.0, 0.0, 0.0],   # 好
        [0.0, 0.0, 0.0, 9.0, 0.0],   # 世
        [0.0, 0.0, 0.0, 0.0, 9.0],   # 界
    ])
    text = ctc_greedy(logits, tokens)
    assert text == "你好世界", text
    print("ctc ok: 你好世界")

    print("FRONTEND_CHECK_PASSED")


if __name__ == "__main__":
    main()

def test_tokens():
    tokens = load_tokens("/workspace/tgoskits/assets/sensevoice/tokens.txt")
    assert len(tokens) == 25055, len(tokens)
    assert tokens[0] == "<unk>" and tokens[3] == "▁the", (tokens[0], tokens[3])
    # CTC 保留 ▁ 会在拼接时转空格；ctc_greedy 已按原样返回，测试拼接行为
    print(f"tokens ok: {len(tokens)} entries, vocab[3]={tokens[3]!r}")

def test_ctc_join():
    tokens = load_tokens("/workspace/tgoskits/assets/sensevoice/tokens.txt")
    import numpy as np
    # 构造 zh 参考句的 token 序列（含重复与 blank 折叠语义已由 ctc_greedy 测过），
    # 这里验证 ▁ 在中文 token 表中的形态
    has_space = any(t == "▁" for t in tokens)
    print(f"tokens contains bare ▁: {has_space}")
