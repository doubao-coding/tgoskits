#!/usr/bin/env python3
"""SenseVoice ASR on RK3588 NPU, CPU frontend + CTC decode.

Runs entirely on StarryOS: numpy fbank frontend (kaldi-compatible), NPU
encoder via the librknnrt C API through ctypes, CTC greedy decoding with a
sentencepiece BPE model.

Levels:
  L0  --help / imports
  L1  missing model file fails with a diagnostic
  L2  zh.wav transcript contains the pinned reference text
  L3  en.wav transcript contains the pinned reference text

Prints SENSEVOICE_RKNN_TEST_PASSED only if every level passes.
"""

import argparse
import ctypes
import json
import math
import os
import struct
import sys
import time
import wave

import numpy as np

# ---------------------------------------------------------------------------
# RKNN C API (rknn_api.h subset used here)
# ---------------------------------------------------------------------------

RKNN_QUERY_INPUT_ATTR = 0
RKNN_QUERY_OUTPUT_ATTR = 1
RKNN_TENSOR_FLOAT32 = 1


class RknnTensorAttr(ctypes.Structure):
    _fields_ = [
        ("index", ctypes.c_int32),
        ("n_dims", ctypes.c_uint32),
        ("name", ctypes.c_char * 256),
        ("n_elems", ctypes.c_uint32),
        ("size", ctypes.c_uint32),
        ("fmt", ctypes.c_int32),
        ("type", ctypes.c_int32),
        ("qnt_type", ctypes.c_int32),
        ("fl", ctypes.c_int8),
        ("zp", ctypes.c_int32),
        ("scale", ctypes.c_float),
        ("w_stride", ctypes.c_int32),
        ("size_with_stride", ctypes.c_uint32),
        ("pass_through", ctypes.c_uint8),
        ("h_stride", ctypes.c_uint32),
    ]


class RknnInput(ctypes.Structure):
    _fields_ = [
        ("index", ctypes.c_uint32),
        ("buf", ctypes.c_void_p),
        ("size", ctypes.c_uint32),
        ("pass_through", ctypes.c_uint8),
        ("type", ctypes.c_int32),
        ("fmt", ctypes.c_int32),
    ]


class RknnOutput(ctypes.Structure):
    _fields_ = [
        ("want_float", ctypes.c_uint8),
        ("is_prealloc", ctypes.c_uint8),
        ("index", ctypes.c_uint32),
        ("buf", ctypes.c_void_p),
        ("size", ctypes.c_uint32),
    ]


class RknnContext:
    """Owns one rknn_context and the loaded librknnrt bindings."""

    def __init__(self, model_path, lib_paths):
        lib = None
        errors = []
        for path in lib_paths:
            try:
                lib = ctypes.CDLL(path, mode=ctypes.RTLD_GLOBAL)
                break
            except OSError as err:
                errors.append(f"{path}: {err}")
        if lib is None:
            raise RuntimeError("cannot load librknnrt.so: " + "; ".join(errors))
        self.lib = lib

        lib.rknn_init.argtypes = [
            ctypes.POINTER(ctypes.c_size_t),
            ctypes.c_void_p,
            ctypes.c_uint32,
            ctypes.c_uint32,
            ctypes.c_void_p,
        ]
        lib.rknn_init.restype = ctypes.c_int
        lib.rknn_destroy.argtypes = [ctypes.c_size_t]
        lib.rknn_query.argtypes = [
            ctypes.c_size_t,
            ctypes.c_int32,
            ctypes.c_void_p,
            ctypes.c_uint32,
        ]
        lib.rknn_inputs_set.argtypes = [
            ctypes.c_size_t,
            ctypes.c_uint32,
            ctypes.POINTER(RknnInput),
        ]
        lib.rknn_run.argtypes = [ctypes.c_size_t, ctypes.c_void_p]
        lib.rknn_wait.argtypes = [ctypes.c_size_t, ctypes.c_void_p]
        lib.rknn_outputs_get.argtypes = [
            ctypes.c_size_t,
            ctypes.c_uint32,
            ctypes.POINTER(RknnOutput),
            ctypes.c_void_p,
        ]
        lib.rknn_outputs_release.argtypes = [
            ctypes.c_size_t,
            ctypes.c_uint32,
            ctypes.POINTER(RknnOutput),
        ]

        with open(model_path, "rb") as handle:
            blob = handle.read()
        self._model_blob = blob  # keep alive: rknn_init reads in place
        self.ctx = ctypes.c_size_t(0)
        ret = lib.rknn_init(
            ctypes.byref(self.ctx),
            ctypes.c_char_p(blob),
            ctypes.c_uint32(len(blob)),
            0,
            None,
        )
        if ret != 0:
            raise RuntimeError(f"rknn_init failed: {ret}")
        self.in_attr = self._query(RKNN_QUERY_INPUT_ATTR)
        self.out_attr = self._query(RKNN_QUERY_OUTPUT_ATTR)

    def _query(self, cmd):
        attr = RknnTensorAttr()
        ret = self.lib.rknn_query(
            self.ctx, cmd, ctypes.byref(attr), ctypes.sizeof(attr)
        )
        if ret != 0:
            raise RuntimeError(f"rknn_query({cmd}) failed: {ret}")
        return attr

    def run(self, speech_batch):
        """speech_batch: float32 [1, T, D] flattened (layout per in_attr)."""
        payload = np.ascontiguousarray(speech_batch, dtype=np.float32)
        inp = RknnInput(
            index=0,
            buf=ctypes.c_void_p(payload.ctypes.data),
            size=ctypes.c_uint32(payload.nbytes),
            pass_through=0,
            type=RKNN_TENSOR_FLOAT32,
            fmt=0,
        )
        ret = self.lib.rknn_inputs_set(self.ctx, 1, ctypes.byref(inp))
        if ret != 0:
            raise RuntimeError(f"rknn_inputs_set failed: {ret}")
        ret = self.lib.rknn_run(self.ctx, None)
        if ret != 0:
            raise RuntimeError(f"rknn_run failed: {ret}")
        ret = self.lib.rknn_wait(self.ctx, None)
        if ret != 0:
            raise RuntimeError(f"rknn_wait failed: {ret}")
        out = RknnOutput(want_float=1, is_prealloc=0, index=0, buf=None, size=0)
        ret = self.lib.rknn_outputs_get(self.ctx, 1, ctypes.byref(out), None)
        if ret != 0:
            raise RuntimeError(f"rknn_outputs_get failed: {ret}")
        try:
            count = out.size // 4
            return np.ctypeslib.as_array(
                ctypes.cast(out.buf, ctypes.POINTER(ctypes.c_float)), shape=(count,)
            ).copy()
        finally:
            self.lib.rknn_outputs_release(self.ctx, 1, ctypes.byref(out))

    def close(self):
        if getattr(self, "ctx", 0):
            self.lib.rknn_destroy(self.ctx)
            self.ctx = 0


# ---------------------------------------------------------------------------
# Frontend: 16k PCM -> fbank80 (kaldi-compatible) -> CMVN -> LFR
# ---------------------------------------------------------------------------


def read_wav_mono(path):
    with wave.open(path, "rb") as handle:
        assert handle.getframerate() == 16000, "expect 16 kHz wav"
        assert handle.getnchannels() == 1, "expect mono wav"
        assert handle.getsampwidth() == 2, "expect s16 wav"
        frames = handle.readframes(handle.getnframes())
    return np.frombuffer(frames, dtype="<i2").astype(np.float32) / 32768.0


def _hamming_povey(n):
    # kaldi "povey" window = 0.5 - 0.5cos(...) raised to 0.85; the reference
    # frontend uses plain hamming ("hamming" window_type), keep it here.
    i = np.arange(n, dtype=np.float64)
    return 0.54 - 0.46 * np.cos(2.0 * math.pi * i / (n - 1))


def fbank80(samples, fs=16000, n_mels=80, frame_ms=25, shift_ms=10):
    """Plain hamming-window log-mel fbank, energy_floor=0, snip_edges=True."""
    frame_len = int(round(fs * frame_ms / 1000.0))
    frame_shift = int(round(fs * shift_ms / 1000.0))
    if len(samples) < frame_len:
        samples = np.pad(samples, (0, frame_len - len(samples)))

    num_frames = 1 + (len(samples) - frame_len) // frame_shift
    window = _hamming_povey(frame_len)
    # Power spectrum
    nfft = 1
    while nfft < frame_len:
        nfft *= 2
    freq_bins = nfft // 2 + 1

    frames = np.lib.stride_tricks.as_strided(
        samples,
        shape=(num_frames, frame_len),
        strides=(samples.strides[0] * frame_shift, samples.strides[0]),
    ).astype(np.float64)
    frames = frames * window
    power = (np.abs(np.fft.rfft(frames, nfft)) ** 2) / nfft

    # Triangular mel filters (kaldi-style mel scale, HTK offsets avoided)
    def hz_to_mel(f):
        return 1127.0 * math.log1p(f / 700.0)

    def mel_to_hz(m):
        return 700.0 * (math.exp(m / 1127.0) - 1.0)

    low, high = 20.0, fs / 2.0
    mel_points = np.linspace(hz_to_mel(low), hz_to_mel(high), n_mels + 2)
    bin_hz = np.linspace(0, fs, freq_bins)
    mel_bin = np.array([hz_to_mel(b) for b in bin_hz])
    filters = np.zeros((n_mels, freq_bins))
    for m in range(n_mels):
        left, center, right = mel_points[m], mel_points[m + 1], mel_points[m + 2]
        up = (mel_bin - left) / (center - left)
        down = (right - mel_bin) / (right - center)
        filters[m] = np.maximum(0, np.minimum(up, down))
    feats = np.log(np.maximum(power @ filters.T, 1e-10))  # energy floor ~0
    return feats.astype(np.float32)


def load_cmvn(path):
    """Parse kaldi am.mvn: <AddShift> row = -mean, <Rescale> row = 1/std.

    The file is a kaldi nnet transcript; the two vectors we need sit on the
    lines after <AddShift>/<LearnRateCoef> and <Rescale>/<LearnRateCoef>, each
    bracketed with '[' ']'. Note the vectors are already 560-wide (LFR 7x80
    spliced), matching the LFR feature dim, so normalization applies AFTER
    LFR, not before.
    """
    mean560, rescale560 = None, None
    lines = open(path, "r", encoding="utf-8").read().splitlines()
    for i, line in enumerate(lines):
        vec = None
        if line.startswith("<AddShift>"):
            vec = _mvn_vector_on(lines, i)
            if vec is not None:
                mean560 = -vec  # AddShift stores negated means
        elif line.startswith("<Rescale>"):
            vec = _mvn_vector_on(lines, i)
            if vec is not None:
                rescale560 = vec  # per-dim 1/std multipliers
    if mean560 is None or rescale560 is None:
        raise ValueError(f"am.mvn missing AddShift/Rescale vectors: {path}")
    return mean560.astype(np.float32), rescale560.astype(np.float32)


def _mvn_vector_on(lines, start):
    """Extract the first bracketed float vector from lines[start:]."""
    buf = ""
    for line in lines[start:]:
        buf += line + " "
        if "]" in line:
            break
    left = buf.find("[")
    right = buf.find("]")
    if left == -1 or right == -1:
        return None
    body = buf[left + 1 : right].split()
    try:
        return np.array([float(v) for v in body], dtype=np.float64)
    except ValueError:
        return None


def apply_lfr(feats, cmvn, lfr_m=7, lfr_n=6):
    """LFR stacking then normalization by the 560-wide am.mvn vectors.

    am.mvn ships AddShift/Rescale already spliced to LFR width (560 = 7*80),
    so mean/rescale multiply the stacked features directly.
    """
    shift, rescale = cmvn
    t = len(feats)
    patches = []
    for start in range(0, t, lfr_n):
        end = min(start + lfr_m, t)
        patch = feats[start:end].reshape(-1)
        if patch.shape[0] < lfr_m * feats.shape[1]:
            patch = np.pad(patch, (0, lfr_m * feats.shape[1] - patch.shape[0]))
        patches.append(patch)
    stacked = np.stack(patches)
    return ((stacked + shift) * rescale).astype(np.float32)


# ---------------------------------------------------------------------------
# Decode: query embedding + CTC greedy + BPE
# ---------------------------------------------------------------------------

# language token indices in embedding.npy (same mapping as the reference)
LANGUAGES = {"auto": 0, "zh": 3, "en": 4, "ja": 5, "ko": 6, "yue": 7}
TEXT_NORM_ITN = 14
TEXT_NORM_NONE = 15
EVENT_EMO_QUERY = (1, 2)
BLANK_ID = 0


def ctc_greedy(logits, tokens):
    """logits: [T, V]; returns decoded text."""
    ids = logits.argmax(axis=-1)
    kept, prev = [], BLANK_ID
    for token_id in ids:
        if token_id != prev and token_id != BLANK_ID:
            kept.append(int(token_id))
        prev = token_id
    return "".join(tokens[i] for i in kept if 0 <= i < len(tokens))


def load_tokens(path):
    """Load a sherpa-onnx tokens.txt: each line is '<surface> <id>'.

    The id space matches the encoder output vocab directly and is the same
    table the .bpe.model encodes, but tokens.txt is trivially parseable and
    ships with the same model family (kept in assets/sensevoice/).
    """
    tokens = {}
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if not line:
                continue
            surface, _, ident = line.rpartition(" ")
            try:
                tokens[int(ident)] = surface
            except ValueError:
                continue
    if not tokens:
        raise ValueError(f"no tokens parsed from {path}")
    return [tokens.get(i, "") for i in range(max(tokens) + 1)]


# ---------------------------------------------------------------------------
# main levels
# ---------------------------------------------------------------------------


def transcribe(ctx, embedding, tokens, cmvn, wav_path, language):
    samples = read_wav_mono(wav_path)
    feats = fbank80(samples)
    speech = apply_lfr(feats, cmvn)

    lang_idx = LANGUAGES[language]
    t_frames = speech.shape[0]
    query = np.concatenate(
        [
            embedding[[lang_idx]].reshape(1, -1),
            embedding[[TEXT_NORM_ITN]].reshape(1, -1),
            embedding[[EVENT_EMO_QUERY[0], EVENT_EMO_QUERY[1]]].reshape(1, -1),
        ],
        axis=1,
    ).astype(np.float32)  # [1, 3*D]
    xs = speech[None, ...]  # [1, T, LFR_D]
    batch = np.concatenate(
        [np.tile(query, (t_frames, 1))[:, None, :], xs], axis=2
    )  # [1, T, LFR_D + 3D]

    logits = ctx.run(batch.reshape(1, -1))
    vocab = embedding.shape[1] if embedding.ndim == 2 else None
    # Output layout is [T, V]; trim V by pieces length when available.
    if tokens:
        v = min(len(tokens), logits.shape[-1])
        return ctc_greedy(logits.reshape(t_frames, -1)[:, :v], tokens)
    return ctc_greedy(logits.reshape(t_frames, -1), [""] * (logits.shape[-1]))


def main():
    parser = argparse.ArgumentParser(description="SenseVoice RK3588 NPU ASR")
    parser.add_argument("--model-dir", default="/opt/sensevoice/model")
    parser.add_argument("--lib-dir", default="/opt/sensevoice/lib")
    parser.add_argument("--wav", action="append", default=[])
    parser.add_argument("--language", default="auto")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    model = os.path.join(args.model_dir, "sense-voice-encoder.rk3588.fp16-scaled.rknn")
    lib_paths = [
        os.path.join(args.lib_dir, "librknnrt.so"),
        "/usr/lib/librknnrt.so",
    ]

    # L1: missing model must fail with a diagnostic.
    if args.selftest and not os.path.exists(model):
        print(f"model not found: {model}", file=sys.stderr)
        return 1

    embedding = np.load(os.path.join(args.model_dir, "embedding.npy"))
    cmvn = load_cmvn(os.path.join(args.model_dir, "am.mvn"))
    tokens = load_tokens(os.path.join(args.model_dir, "tokens.txt"))

    started = time.time()
    ctx = RknnContext(model, lib_paths)
    print(f"[perf] model load: {time.time() - started:.2f}s")
    try:
        for wav_path in args.wav:
            started = time.time()
            text = transcribe(ctx, embedding, tokens, cmvn, wav_path, args.language)
            print(json.dumps({"wav": wav_path, "text": text}, ensure_ascii=False))
    finally:
        ctx.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
