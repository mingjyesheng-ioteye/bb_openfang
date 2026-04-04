#!/usr/bin/env bash
set -euo pipefail

echo "Running coding latency benchmark harnesses..."

CAPTURE_ARGS=(--ignored --nocapture)
if [[ "${1:-}" == "--no-capture" ]]; then
  CAPTURE_ARGS=(--ignored)
fi

echo "[1/3] Synthetic parallel batch benchmark"
cargo test -p openfang-runtime --lib benchmark_parallel_read_only_batch_latency -- "${CAPTURE_ARGS[@]}"

echo "[2/3] Scenario search benchmark"
cargo test -p openfang-runtime --lib benchmark_search_tools_parallel_vs_sequential -- "${CAPTURE_ARGS[@]}"

echo "[3/3] Long-session compaction benchmark"
cargo test -p openfang-runtime --lib benchmark_compaction_long_session_latency -- "${CAPTURE_ARGS[@]}"

echo "Benchmark harness run complete."
