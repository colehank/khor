#!/usr/bin/env bash
# W7 · the same load profile as scripts/windows-perf.ps1, on the other platform.
#
# APPROVAL-PACKAGE §5.2 wants Linux/macOS as the baseline beside Windows, with
# the same HEAD, the same locked dependencies and the same workload. This is that
# half: identical metric list, identical warmup/run counts, median/P95/peak.
#
# Two metrics cannot be compared like-for-like and are reported as such rather
# than quietly dropped:
#   - gui_cold_start has no honest counterpart on a machine with no desktop
#     session; §5.2's GUI row is measured where a window can be shown.
#   - the file scan uses `khor ls <this host> <dir>`, which on Linux goes through
#     the same node code path as on Windows, so that one IS comparable.
#
# Usage:
#   bash scripts/windows-perf.sh -b <khor> -s <store> [-r runs] [-w warmup] [-o json]
set -euo pipefail

KHOR=""
STORE=""
RUNS=5
WARMUP=2
OUT=""

while [ $# -gt 0 ]; do
  case "$1" in
    -b) KHOR="$2"; shift 2 ;;
    -s) STORE="$2"; shift 2 ;;
    -r) RUNS="$2"; shift 2 ;;
    -w) WARMUP="$2"; shift 2 ;;
    -o) OUT="$2"; shift 2 ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

[ -n "$KHOR" ] || { echo "need -b <khor>" >&2; exit 2; }
[ -x "$KHOR" ] || { echo "not executable: $KHOR" >&2; exit 2; }
[ -n "$STORE" ] || STORE="$(dirname "$KHOR")/../perf-home"
mkdir -p "$STORE"
ROOT="$(dirname "$STORE")"
mkdir -p "$ROOT"
export KHOR_HOME="$STORE"
export KHOR_RUNTIME_DIR="$ROOT/runtime"

FIXTURE="$ROOT/perf-files"
if [ ! -d "$FIXTURE" ]; then
  mkdir -p "$FIXTURE"
  for i in $(seq 0 39); do
    f=$(printf "%s/f%02d.txt" "$FIXTURE" "$i")
    : > "$f"
    for _ in $(seq 1 400); do printf 'khor perf line %s\n' "$i" >> "$f"; done
  done
fi

# Milliseconds, one measured run per call. `date +%s%N` is what coreutils gives.
now_ms() { date +%s%3N; }

workload() {
  case "$1" in
    cli_version)    "$KHOR" version >/dev/null 2>&1 ;;
    cli_doctor)     "$KHOR" doctor --json >/dev/null 2>&1 ;;
    # **A command that exits by itself**, as on the Windows side: a plain `sh`
    # with no tty waits for input and measures nothing.
    session_create) "$KHOR" open -d -- sh -c 'exit 0' >/dev/null 2>&1 ;;
    sessions_list)  "$KHOR" sessions >/dev/null 2>&1 ;;
    file_scan)      "$KHOR" ls "$(hostname)" "$FIXTURE" >/dev/null 2>&1 ;;
    file_copy)
      rm -rf "$ROOT/perf-copy"
      cp -r "$FIXTURE" "$ROOT/perf-copy" ;;
    *) return 1 ;;
  esac
}

METRICS="cli_version cli_doctor session_create sessions_list file_scan file_copy"
echo "khor perf (this host) · $KHOR"
echo "store $STORE · runs $RUNS (+$WARMUP warmup)"
[ -n "$OUT" ] && printf '{\n' > "$OUT"
first=1
for m in $METRICS; do
  samples="$ROOT/.samples.$m"
  : > "$samples"
  failed=""
  total=$((WARMUP + RUNS))
  i=0
  while [ $i -lt $total ]; do
    start=$(now_ms)
    if workload "$m"; then
      end=$(now_ms)
      if [ $i -ge "$WARMUP" ]; then echo $((end - start)) >> "$samples"; fi
    else
      failed="failed at run $((i + 1))"
      break
    fi
    i=$((i + 1))
  done
  if [ -n "$failed" ] || [ ! -s "$samples" ]; then
    echo "  $(printf '%-22s' "$m") $failed"
    [ -n "$OUT" ] && { [ $first -eq 0 ] && printf ',\n' >> "$OUT"; first=0
      printf '  "%s": {"status":"FAILED","error":"%s"}' "$m" "${failed:-no samples}" >> "$OUT"; }
    continue
  fi
  read -r median p95 peak count <<EOF
$(sort -n "$samples" | awk '{v[NR]=$1} END {
  n=NR;
  m=(n%2==1)?v[(n+1)/2]:int((v[n/2]+v[n/2+1])/2);
  p=v[int(n*0.95)]; if (p=="") p=v[n];
  printf "%d %d %d %d\n", m, p, v[n], n}')
EOF
  echo "  $(printf '%-22s' "$m") median ${median} ms   p95 ${p95}   peak ${peak}"
  [ -n "$OUT" ] && { [ $first -eq 0 ] && printf ',\n' >> "$OUT"; first=0
    printf '  "%s": {"status":"OK","unit":"ms","runs":%s,"median":%s,"p95":%s,"peak":%s}' \
      "$m" "$count" "$median" "$p95" "$peak" >> "$OUT"; }
  rm -f "$samples"
done
[ -n "$OUT" ] && { printf '\n}\n' >> "$OUT"; echo "written: $OUT"; }
echo ""
echo "NOTE: gui_cold_start has no counterpart on a host with no desktop session;"
echo "      that row of §5.2 is measured where a window can actually be shown."
