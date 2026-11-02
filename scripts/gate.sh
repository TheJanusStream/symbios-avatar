#!/usr/bin/env bash
# This crate's gate, in less time.
#
#   scripts/gate.sh                run what the tree has not yet passed
#   scripts/gate.sh --viewer       also the viewer's tests and clippy against this tree,
#                                  ../bevy_symbios_avatar's Cargo.lock put back after
#   scripts/gate.sh --libm         also a chosen set of tests with every libm function a
#                                  last place or two off, as CI's differs
#   scripts/gate.sh --force        run every step, passed or not
#
# It holds the entries of .github/workflows/rust.yml but one: the suite with no features,
# which a run with every feature proves unless a test leans on a feature without gating it.
#
# Steps that share no build lock run side by side. A step that passed is not run again
# while everything that can decide it is the same: every file git would see, the
# toolchain, the command and the flags. The record lives in target/gate/, so losing
# target/ only costs the time.
#
# It says `gate: passed` only when every step it owes has reported a pass and nothing it
# read has changed under it: a lane that dies, or an edit mid-run, reads as a failure.
#
# GATE_JOBS (24), GATE_MEM_FLOOR (8 GiB) and GATE_VIEWER (../bevy_symbios_avatar) tune it.
set -u
# What `step` reads of its callers here: from outside, a record would be keyed on it.
unset STEP_PRINT
root=$(git rev-parse --show-toplevel) || exit 2
cd "$root" || exit 2
jobs=${GATE_JOBS:-24}
floor=${GATE_MEM_FLOOR:-8}
force=0 viewer=0 libm=0
for arg in "$@"; do
    case $arg in
        --force) force=1 ;;
        --viewer) viewer=1 ;;
        --libm) libm=1 ;;
        *) sed -n '2,/^[^#]/p' "$0" | sed '$d'; exit 2 ;;
    esac
done
gate=target/gate
logs=$gate/logs
passed=$gate/passed
viewer_tree=${GATE_VIEWER:-../bevy_symbios_avatar}
mkdir -p "$logs" "$passed" || exit 2
results=$(mktemp -d) || exit 2
trap 'rm -rf "$results"' EXIT
touch "$results/report"
started=$(date +%s)

# One run at a time, as a second would read the first's lockfile. A step left behind by
# a run that was killed holds this too.
exec 9> "$gate/lock"
if ! flock -n 9; then
    echo "gate: another run, or a step one left behind, is still going" >&2
    exit 2
fi

# Where --viewer keeps the viewer's lockfile, with its time. Out of the temporary
# directory, so that a run killed outright leaves it for the next one to put back.
kept=$root/$gate/viewer.lock.kept
put_back_lock() {
    if [ -e "$kept" ]; then
        mv -f "$kept" "$viewer_tree/Cargo.lock"
    fi
}
put_back_lock
trap 'put_back_lock; rm -rf "$results"' EXIT

# Stopped: the steps first, or one could write the lockfile after it was put back.
stopped() {
    trap '' INT TERM
    kill $(jobs -p) 2>/dev/null
    if [ -e "$results/guards" ]; then
        local guard
        while read -r guard; do kill "$guard" 2>/dev/null; done < "$results/guards"
        while read -r guard; do
            while kill -0 "$guard" 2>/dev/null; do sleep 0.2; done
        done < "$results/guards"
    fi
    echo "gate: stopped" >&2
    exit 130
}
trap stopped INT TERM

# What every file in `dir` that git would see holds, as a list of names and hashes. In a
# fixed order: ls-files moves a file when it is staged, which changes nothing it holds.
tree_of() (
    cd "$1" || exit 1
    git ls-files -zco --exclude-standard | LC_ALL=C sort -zu | while IFS= read -r -d '' file; do
        [ -f "$file" ] && printf '%s\n' "$file"
    done > "$results/list"
    paste "$results/list" <(git hash-object --stdin-paths < "$results/list")
)

print_of() {
    {
        tree_of .
        [ -f Cargo.lock ] && sha256sum Cargo.lock
        rustc -vV
        cargo -V
        env | grep -E '^(RUSTFLAGS|RUSTDOCFLAGS|RUSTC_[A-Z0-9_]*|CARGO_[A-Z0-9_]*|[A-Z0-9_]*BLESS[A-Z0-9_]*|SKIP_SUBMENTAL)=' | sort
        [ -f ~/.cargo/config.toml ] && sha256sum ~/.cargo/config.toml
    } | sha256sum | cut -c1-32
}

viewer_print_of() {
    { echo "$1"; tree_of "$viewer_tree"; sha256sum "$viewer_tree/Cargo.lock"; } | sha256sum | cut -c1-32
}

# The viewer against this tree. A patch whose version does not match is silently ignored,
# so check it is in the build. A directory git does not hold would hash as empty.
patch="patch.crates-io.symbios-avatar.path=\"$root\""
if [ "$viewer" = 1 ]; then
    if [ ! -e "$viewer_tree/.git" ]; then
        echo "gate: $viewer_tree is not a git checkout, so the viewer cannot run" >&2
        exit 2
    fi
    [ -f "$viewer_tree/Cargo.lock" ] || (cd "$viewer_tree" && cargo generate-lockfile > /dev/null 2>&1)
    cp -p "$viewer_tree/Cargo.lock" "$kept" || exit 2
    (cd "$viewer_tree" && cargo update -p symbios-avatar --config "$patch" > /dev/null 2>&1)
    if ! (cd "$viewer_tree" && cargo tree -p symbios-avatar --depth 0 --config "$patch" 2>/dev/null) | head -1 | grep -q '(/'; then
        echo "gate: the patch is not in the viewer's build: read its Cargo.lock" >&2
        exit 2
    fi
fi

[ -f Cargo.lock ] || cargo generate-lockfile > /dev/null 2>&1
fingerprint=$(print_of)
viewer_print=$fingerprint
if [ "$viewer" = 1 ]; then
    viewer_print=$(viewer_print_of "$fingerprint")
fi

# Every step this run owes: a lane that dies takes its steps with it, and a step that
# never reported reads as a failure.
owed=(fmt tests clippy clippyserde clippyclips clippyall docprivate docpublic nofeat wasm)
[ "$viewer" = 1 ] && owed+=(viewertests viewerclippy)
[ "$libm" = 1 ] && owed+=(libm)
[ -f scripts/gate_test.sh ] && owed+=(selftest)

# guarded COMMAND...: under the memory guard, and where a stopped run can reach it.
guarded() {
    sh "$root/scripts/memguard.sh" "$floor" "$@" &
    local guard=$!
    echo "$guard" >> "$results/guards"
    wait "$guard"
}

# step NAME COMMAND...: the guard, the record, one line of report.
step() {
    local name=$1; shift
    local print=${STEP_PRINT:-$fingerprint}
    local record=$root/$passed/$name-$print-$(printf '%s' "$*" | sha256sum | cut -c1-12)
    if [ "$force" = 0 ] && [ -e "$record" ]; then
        echo "$name: passed already" >> "$results/report"
        touch "$results/ok-$name"
        return 0
    fi
    local begun=$(date +%s) code
    guarded env CARGO_BUILD_JOBS="$jobs" "$@" > "$root/$logs/$name.log" 2>&1
    code=$?
    local took=$(( $(date +%s) - begun ))
    if [ "$code" = 0 ]; then
        touch "$record" "$results/ok-$name"
        echo "$record" >> "$results/records"
        echo "$name: passed in $took s" >> "$results/report"
    else
        echo "$name: FAILED with $code in $took s, see $logs/$name.log" >> "$results/report"
        touch "$results/failed"
    fi
    return "$code"
}

libm_failed() {
    { echo "libm: FAILED: $1"; } >> "$results/report"
    touch "$results/failed"
    return 1
}

# Every direction and size of nudge a process, side by side, with what each must print.
nudge() {
    local record=$root/$passed/libm-$fingerprint
    if [ "$force" = 0 ] && [ -e "$record" ]; then
        echo "libm: passed already" >> "$results/report"
        touch "$results/ok-libm"
        return 0
    fi
    # Built now and named by this build: a log's names can outlive what they named.
    guarded env CARGO_BUILD_JOBS="$jobs" cargo test --release --all-features --no-run \
        > "$logs/libm-build.log" 2>&1 || { libm_failed "the tests did not build"; return 1; }
    (cd tools/libm-nudge && cargo build --release --quiet) \
        > "$logs/libm-shim.log" 2>&1 || { libm_failed "the shim did not build"; return 1; }
    local shim=$root/tools/libm-nudge/target/release/liblibm_nudge.so
    local lib int
    lib=$(sed 's/\x1b\[[0-9;]*m//g' "$logs/libm-build.log" | grep -oE '[^ (]*/deps/symbios_avatar-[0-9a-f]+' | head -1)
    int=$(sed 's/\x1b\[[0-9;]*m//g' "$logs/libm-build.log" | grep -oE '[^ (]*/deps/integration-[0-9a-f]+' | head -1)
    if [ -z "$lib" ] || [ -z "$int" ]; then
        libm_failed "no test binaries named in $logs/libm-build.log"
        return 1
    fi
    local dir ulps
    local runs=()
    for dir in up down; do
        for ulps in 1 2; do
            runs+=("$dir$ulps-lib" "$dir$ulps-int")
            # avatar:: since #411: hashes of a built beast pinned there held on this
            # machine's glibc and on no other.
            (
                guarded env SHIM_FN=all SHIM_DIR=$dir SHIM_ULPS=$ulps LD_PRELOAD="$shim" "$lib" \
                    anim:: face::beast:: uv:: texture::coat:: plan:: record:: hair::fur:: cage:: rig::junction avatar:: \
                    > "$logs/libm-$dir$ulps-lib.log" 2>&1
                echo $? > "$results/libm-$dir$ulps-lib"
                guarded env SHIM_FN=all SHIM_DIR=$dir SHIM_ULPS=$ulps LD_PRELOAD="$shim" "$int" \
                    driver:: rest:: uv:: budget:: lexicon:: \
                    > "$logs/libm-$dir$ulps-int.log" 2>&1
                echo $? > "$results/libm-$dir$ulps-int"
            ) &
        done
    done
    wait
    # A failing test ends a binary with 101 and its summary. Anything else, a crash, a
    # binary that is not there, a kill, is what this instrument exists to find.
    local run code
    for run in "${runs[@]}"; do
        code=$(cat "$results/libm-$run" 2>/dev/null)
        if { [ "$code" != 0 ] && [ "$code" != 101 ]; } || ! grep -q '^test result:' "$logs/libm-$run.log"; then
            libm_failed "$run ended with ${code:-nothing} and no summary, see $logs/libm-$run.log"
            return 1
        fi
    done
    # Exact-equality tests that compare two paths through libm fail under a nudge of
    # one path, on every tree: read what is new against this list.
    local known='anim::turn::tests::a_straight_turn_is_the_walk_that_was_already_there|anim::foothold::tests::a_steady_walk_that_turns_is_the_stateless_walk|anim::foothold::tests::a_held_walk_at_constant_speed_is_the_stateless_walk'
    local new
    new=$(for run in "${runs[@]}"; do cat "$logs/libm-$run.log"; done | grep -E '^test .*FAILED$' | grep -vE "$known" | sort -u)
    if [ -n "$new" ]; then
        libm_failed "new under a nudge:"
        echo "$new" >> "$results/report"
        return 1
    fi
    touch "$record" "$results/ok-libm"
    echo "$record" >> "$results/records"
    echo "libm: nothing new under a nudge" >> "$results/report"
}

step fmt cargo fmt --all -- --check

if [ -f scripts/gate_test.sh ]; then
    (   # What this script itself must do, on stubs, whenever a script has changed.
        STEP_PRINT=$(cat scripts/* | sha256sum | cut -c1-32) step selftest bash scripts/gate_test.sh
    ) &
fi
(   # Tests. `--no-fail-fast` so one failure does not hide the next.
    step tests cargo test --release --all-features --no-fail-fast
    if [ "$libm" = 1 ]; then
        nudge
    fi
) &
(   # Debug-profile checks, which share a lock. Rustdoc merges each run into target/doc
    # and grows without limit, so it starts from an empty one (overlands #1689).
    step clippy cargo clippy --all-targets -- -D warnings
    step clippyserde cargo clippy --all-targets --features serde-avatar -- -D warnings
    step clippyclips cargo clippy --all-targets --features builtin-clips -- -D warnings
    step clippyall cargo clippy --all-targets --all-features -- -D warnings
    cargo clean --doc > /dev/null 2>&1
    RUSTDOCFLAGS='-D warnings' step docprivate cargo doc --no-deps --document-private-items --all-features
    cargo clean --doc > /dev/null 2>&1
    RUSTDOCFLAGS='-D warnings' step docpublic cargo doc --no-deps --all-features
    cargo clean --doc > /dev/null 2>&1
    RUSTFLAGS='-D warnings' step nofeat cargo check --no-default-features
) &
(   # The web's.
    RUSTFLAGS='-D warnings' step wasm cargo check --target wasm32-unknown-unknown --lib --all-features
) &
if [ "$viewer" = 1 ]; then
    (
        cd "$viewer_tree" || exit 1
        STEP_PRINT=$viewer_print step viewertests cargo test --release --config "$patch"
        STEP_PRINT=$viewer_print step viewerclippy cargo clippy --all-targets --config "$patch" -- -D warnings
    ) &
fi
wait

# Only what still describes what was read counts.
bad=0
now=$(print_of)
if [ "$now" != "$fingerprint" ] || { [ "$viewer" = 1 ] && [ "$(viewer_print_of "$now")" != "$viewer_print" ]; }; then
    echo "gate: the tree, a lockfile or the tools changed during this run: its passes do not count" >> "$results/report"
    [ -e "$results/records" ] && xargs -r rm -f < "$results/records"
    bad=1
fi
[ -e "$results/failed" ] && bad=1
for name in "${owed[@]}"; do
    if [ ! -e "$results/ok-$name" ]; then
        grep -q "^$name: FAILED" "$results/report" || echo "$name: did not report" >> "$results/report"
        bad=1
    fi
done

cat "$results/report"
echo "gate: $(( $(date +%s) - started )) s"
if [ "$bad" = 1 ]; then
    echo "gate: FAILED"
    exit 1
fi
echo "gate: passed"
