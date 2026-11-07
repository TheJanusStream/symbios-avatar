#!/usr/bin/env bash
# People and beasts driven through one scenario on this tree against HEAD's, frame by
# frame and bit for bit (#424): stand, walk, trot or run, gallop or sprint, a curve, a
# stop, a pivot and an idle, at the engine's reference frame time, 1/60 s.
#
#   scripts/compare-motion.sh [--hz N] [--scenario S]
#
# S is examples/motiondump's: `all` (the default), `slope`, `gestures`, `pivot`, `idle`.
#
# Exits 0 when no frame of any body moved and 1 when one did, naming each body and the
# first frame it moved in. HEAD's dump is kept per commit, rate and probe under
# target/baseline-motion/ (compare-published.sh clears target/baseline/), so a rerun
# costs one dump.
set -eu
root=$(git rev-parse --show-toplevel); cd "$root"
hz=60
scenario=all
while [ $# -gt 0 ]; do
    case $1 in
        --hz) hz=$2; shift 2 ;;
        --scenario) scenario=$2; shift 2 ;;
        *) echo "unknown argument $1" >&2; exit 2 ;;
    esac
done
dump=$hz-$scenario
head=$(git rev-parse --short HEAD)-$(sha256sum examples/motiondump.rs | cut -c1-8)
base=$root/target/baseline-motion/$head
if [ ! -s "$base/old-$dump.dump" ]; then
    mkdir -p "$base"
    if [ ! -d "$base/tree" ]; then
        mkdir -p "$base/tree"
        git archive HEAD | tar -x -C "$base/tree"
        [ -f Cargo.lock ] && cp Cargo.lock "$base/tree/"
    fi
    # A commit from before this script has no probe of its own: lend it this tree's.
    mkdir -p "$base/tree/examples"
    cp examples/motiondump.rs "$base/tree/examples/motiondump.rs"
    (cd "$base/tree" && CARGO_TARGET_DIR=$base/target cargo build --release --example motiondump --quiet)
    "$base/target/release/examples/motiondump" "$base/old-$dump.dump" --hz "$hz" --scenario "$scenario"
fi
cargo build --release --example motiondump --quiet
target/release/examples/motiondump "$base/new-$dump.dump" --hz "$hz" --scenario "$scenario"
trap 'rm -f "$base/new-$dump.dump"' EXIT
python3 - "$base/old-$dump.dump" "$base/new-$dump.dump" <<'PY'
import collections, itertools, sys
same = collections.Counter(); moved = collections.Counter(); first = {}
lines = unpaired = 0
with open(sys.argv[1]) as old, open(sys.argv[2]) as new:
    for was, now in itertools.zip_longest(old, new):
        if was is None or now is None:
            unpaired += 1
            continue
        lines += 1
        body = was.split(" ", 1)[0]
        if was == now:
            same[body] += 1
        else:
            moved[body] += 1
            first.setdefault(body, was.split()[1])
for body in sorted(set(same) | set(moved)):
    note = f"  first moved at frame {first[body]}" if body in first else ""
    print(f"{body:18} same {same[body]:5} moved {moved[body]:5}{note}")
print("frames", lines, "unpaired", unpaired)
if lines == 0 or unpaired or moved:
    print("verdict: DIFFERENT" if lines else "verdict: nothing was read")
    sys.exit(1)
print("verdict: identical")
PY
