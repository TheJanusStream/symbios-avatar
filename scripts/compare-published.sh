#!/usr/bin/env bash
# Sixteen seeds of people and beasts on this tree against HEAD's, line by line, and never
# keyed on a line's first words: that collapsed most rows once and read none of them.
#
#   scripts/compare-published.sh
#
# Exits 0 when nothing moved and 1 when anything did. Whether it may have is the reader's
# to say, from the table: people should match to the byte; a beast's body and fur move
# when its skin does, its code and record when what it wears does. The generator stamp is
# ignored. HEAD's dump is kept per commit and dump, under target/baseline/, so a rerun
# costs one dump.
set -eu
root=$(git rev-parse --show-toplevel); cd "$root"
head=$(git rev-parse --short HEAD)-$(sha256sum examples/fleetdump.rs | cut -c1-8)
base=$root/target/baseline/$head
if [ ! -s "$base/old.dump" ]; then
    rm -rf "$root/target/baseline"; mkdir -p "$base/tree"
    git archive HEAD | tar -x -C "$base/tree"
    [ -f Cargo.lock ] && cp Cargo.lock "$base/tree/"
    # A commit from before this script has no dump of its own: lend it this tree's.
    mkdir -p "$base/tree/examples"
    cp examples/fleetdump.rs "$base/tree/examples/fleetdump.rs"
    (cd "$base/tree" && CARGO_TARGET_DIR=$base/target cargo build --release --features serde-avatar --example fleetdump --quiet)
    "$base/target/release/examples/fleetdump" "$base/old.dump"
fi
cargo build --release --features serde-avatar --example fleetdump --quiet
target/release/examples/fleetdump "$base/new.dump"
trap 'rm -f "$base/new.dump"' EXIT
python3 - "$base/old.dump" "$base/new.dump" <<'PY'
import collections, itertools, re, sys
rows = collections.defaultdict(lambda: [0, 0])
seeds = collections.Counter()
stamp = lambda line: re.sub(r'"generator":\s*\d+', '"generator":0', line)
lines = unpaired = moved = 0
with open(sys.argv[1], errors="replace") as old, open(sys.argv[2], errors="replace") as new:
    for was, now in itertools.zip_longest(old, new):
        if was is None or now is None:
            unpaired += 1
            continue
        lines += 1
        words = was.split(" ", 4)
        if words[0] == "beast":
            label = "beast " + words[3]
        elif words[0] == "person":
            label = "person " + words[2]
        else:
            label = words[0]
        same = stamp(was) == stamp(now)
        rows[label][0 if same else 1] += 1
        if not same and label != "beast stamp":
            moved += 1
            if words[0] == "beast":
                seeds[words[1]] += 1
for label in sorted(rows):
    print(f"{label:16} same {rows[label][0]:4} different {rows[label][1]:4}")
print("lines", lines, "unpaired", unpaired, "beast seeds that differ:", sorted(seeds, key=int))
if lines == 0 or unpaired or moved:
    print("verdict: DIFFERENT" if lines else "verdict: nothing was read")
    sys.exit(1)
print("verdict: identical")
PY
