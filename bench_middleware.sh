#!/usr/bin/env bash
set -euo pipefail

if ! command -v hey >/dev/null 2>&1; then
  printf "error: 'hey' is required (https://github.com/rakyll/hey)\n" >&2
  exit 1
fi

TOTAL_REQUESTS="${TOTAL_REQUESTS:-2000}"
CONCURRENCY="${CONCURRENCY:-50}"
BASE_URL="${BASE_URL:-http://127.0.0.1:3000}"
TRACEPARENT_VALUE="${TRACEPARENT_VALUE:-00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01}"

SERVER_LOG="${SERVER_LOG:-/tmp/axum-tracing-bench.log}"

cargo run >"${SERVER_LOG}" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "${SERVER_PID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in {1..100}; do
  if curl -sSf "${BASE_URL}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

printf "\n== Warmup ==\n"
hey -n 300 -c 20 "${BASE_URL}/health"

printf "\n== Run 1: no traceparent request header ==\n"
hey -n "${TOTAL_REQUESTS}" -c "${CONCURRENCY}" "${BASE_URL}/"

printf "\n== Run 2: with traceparent request header ==\n"
hey -n "${TOTAL_REQUESTS}" -c "${CONCURRENCY}" -H "traceparent: ${TRACEPARENT_VALUE}" "${BASE_URL}/"

printf "\nServer log: %s\n" "${SERVER_LOG}"
