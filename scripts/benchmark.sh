#!/usr/bin/env bash
#
# Head-to-head hyperfine comparison of `rum -rf` against `/bin/rm -rf`.
#
# Usage: scripts/benchmark.sh [--runs N] [--warmup N] [--out DIR]
#
# Each scenario builds a fresh fixture per iteration via hyperfine's
# `--prepare`, so timing covers the deletion only. Results print to stdout
# and are also written as Markdown + JSON under $OUT (default: ./bench-results).

set -euo pipefail

RUNS=20
WARMUP=3
OUT="bench-results"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --runs)   RUNS=$2;   shift 2 ;;
        --warmup) WARMUP=$2; shift 2 ;;
        --out)    OUT=$2;    shift 2 ;;
        -h|--help)
            sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

command -v hyperfine >/dev/null || {
    echo "hyperfine not found. Install with: brew install hyperfine" >&2
    exit 1
}

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO_ROOT"

echo ">>> Building release binary"
cargo build --release --quiet

RUM="$REPO_ROOT/target/release/rum"
RM="/bin/rm"

SCRATCH=$(mktemp -d -t rum-bench.XXXXXX)
mkdir -p "$OUT"
trap 'rm -rf "$SCRATCH"' EXIT

prep_dir() {
    # Portable "populate TRG with N files of SIZE bytes"
    local trg=$1 n=$2 size=$3
    rm -rf "$trg"
    mkdir -p "$trg"
    local i=1
    if [[ "$size" -eq 0 ]]; then
        while [[ $i -le $n ]]; do : > "$trg/f$i"; i=$((i + 1)); done
    else
        while [[ $i -le $n ]]; do
            head -c "$size" /dev/zero > "$trg/f$i"
            i=$((i + 1))
        done
    fi
}

prep_tree() {
    local trg=$1 depth=$2 wide=$3 files_per_dir=$4 size=$5
    rm -rf "$trg"
    _mk_tree "$trg" "$depth" "$wide" "$files_per_dir" "$size"
}

_mk_tree() {
    local d=$1 depth=$2 wide=$3 files=$4 size=$5
    mkdir -p "$d"
    local i=1
    while [[ $i -le $files ]]; do
        head -c "$size" /dev/zero > "$d/f$i"
        i=$((i + 1))
    done
    [[ $depth -eq 0 ]] && return
    local w=1
    while [[ $w -le $wide ]]; do
        _mk_tree "$d/d$w" $((depth - 1)) "$wide" "$files" "$size"
        w=$((w + 1))
    done
}

prep_large_file() {
    local trg=$1 size=$2
    rm -f "$trg"
    mkfile_portable "$trg" "$size"
}

mkfile_portable() {
    local out=$1 size=$2
    # `mkfile` exists on macOS, `fallocate` on Linux. Fall back to `dd`.
    if command -v mkfile >/dev/null 2>&1; then
        mkfile -n "$size" "$out" >/dev/null
    elif command -v fallocate >/dev/null 2>&1; then
        fallocate -l "$size" "$out"
    else
        dd if=/dev/zero of="$out" bs=1048576 count=$((size / 1048576)) status=none
    fi
}

export -f prep_dir prep_tree _mk_tree prep_large_file mkfile_portable

run_scenario() {
    local name=$1 prepare=$2 rum_args=$3 rm_args=$4
    local target="$SCRATCH/$name"
    echo
    echo "============================================================"
    echo "  $name"
    echo "============================================================"
    hyperfine \
        --shell=none \
        --warmup "$WARMUP" \
        --runs "$RUNS" \
        --prepare "bash -c '$prepare' _ $target" \
        --export-markdown "$OUT/$name.md" \
        --export-json "$OUT/$name.json" \
        --command-name "rum $rum_args" "$RUM $rum_args $target" \
        --command-name "rm $rm_args"  "$RM  $rm_args $target"
}

# --- Scenarios --------------------------------------------------------------

run_scenario "flat-10000-empty" \
    "prep_dir \"\$1\" 10000 0" "-rf" "-rf"

run_scenario "flat-2000x8KiB" \
    "prep_dir \"\$1\" 2000 8192" "-rf" "-rf"

run_scenario "flat-500x64KiB" \
    "prep_dir \"\$1\" 500 65536" "-rf" "-rf"

run_scenario "tree-3w5d10f-1KiB" \
    "prep_tree \"\$1\" 5 3 10 1024" "-rf" "-rf"

run_scenario "tree-4w3d20f-4KiB" \
    "prep_tree \"\$1\" 3 4 20 4096" "-rf" "-rf"

run_scenario "single-large-1GiB" \
    "prep_large_file \"\$1\" 1g" "-f" "-f"

# --- Summary ----------------------------------------------------------------

echo
echo "============================================================"
echo "  Summary (results in $OUT/)"
echo "============================================================"
for md in "$OUT"/*.md; do
    echo
    echo "-- $(basename "$md" .md) --"
    cat "$md"
done
