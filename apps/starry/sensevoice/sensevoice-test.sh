#!/bin/sh

# SenseVoice inference levels for the StarryOS sensevoice app.
# Prints SENSEVOICE_TEST_PASSED only when every level succeeds.

BIN=/opt/sensevoice/bin/sherpa-onnx-offline
MODEL_DIR=/opt/sensevoice/model
LOG_DIR=/tmp/sensevoice

mkdir -p "$LOG_DIR" 2>/dev/null || true

# L0: the binary must execute (dynamic loader + glibc libs present).
if ! "$BIN" --help >"$LOG_DIR/help.log" 2>&1; then
    echo "SENSEVOICE_TEST_FAILED: L0 help rc=$?"
    exit 1
fi

# L1: a missing model must fail gracefully with a diagnostic.
"$BIN" --sense-voice-model=/nonexistent.onnx --tokens="$MODEL_DIR/tokens.txt" \
    "$MODEL_DIR/zh.wav" >"$LOG_DIR/missing-model.log" 2>&1
missing_rc=$?
if [ "$missing_rc" -eq 0 ]; then
    echo "SENSEVOICE_TEST_FAILED: L1 missing model returned 0"
    exit 1
fi
if ! grep -qiE "error|fail|not exist|no such|cannot|unable" "$LOG_DIR/missing-model.log"; then
    echo "SENSEVOICE_TEST_FAILED: L1 missing model printed no diagnostic"
    exit 1
fi

# L2: Chinese reference clip. The transcript substring is pinned from a
# native-host run of the same binary and model.
"$BIN" --sense-voice-model="$MODEL_DIR/model.int8.onnx" \
    --tokens="$MODEL_DIR/tokens.txt" --num-threads=1 \
    "$MODEL_DIR/zh.wav" >"$LOG_DIR/zh.out" 2>"$LOG_DIR/zh.log"
zh_rc=$?
if [ "$zh_rc" -ne 0 ]; then
    echo "SENSEVOICE_TEST_FAILED: L2 zh rc=$zh_rc"
    exit 1
fi
if ! grep -q "开饭时间早上九点至下午五点" "$LOG_DIR/zh.out"; then
    echo "SENSEVOICE_TEST_FAILED: L2 zh transcript mismatch"
    cat "$LOG_DIR/zh.out"
    exit 1
fi

# L3: English reference clip.
"$BIN" --sense-voice-model="$MODEL_DIR/model.int8.onnx" \
    --tokens="$MODEL_DIR/tokens.txt" --num-threads=1 \
    "$MODEL_DIR/en.wav" >"$LOG_DIR/en.out" 2>"$LOG_DIR/en.log"
en_rc=$?
if [ "$en_rc" -ne 0 ]; then
    echo "SENSEVOICE_TEST_FAILED: L3 en rc=$en_rc"
    exit 1
fi
if ! grep -q "the tribal chieftain" "$LOG_DIR/en.out"; then
    echo "SENSEVOICE_TEST_FAILED: L3 en transcript mismatch"
    cat "$LOG_DIR/en.out"
    exit 1
fi

grep -q "Real time factor" "$LOG_DIR/zh.log" && grep "Real time factor" "$LOG_DIR/zh.log"
echo "SENSEVOICE_TEST_PASSED"
