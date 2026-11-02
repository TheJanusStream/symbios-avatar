#!/usr/bin/env bash
# What this repo's scripts must never do, on a stub toolchain: say `passed` over a check
# that failed or did not run, skip one they owe, or leave a patched lockfile behind. Each
# case builds its own small world, side by side, and no real cargo runs.
#
#   scripts/gate_test.sh                   every case
#   scripts/gate_test.sh CASE...           just those
#
# GATE_UNDER_TEST names another copy of the scripts, to see a case fail without a fix.
set -u
scripts=${GATE_UNDER_TEST:-$(cd "$(dirname "$0")" && pwd)}

new_world() {
    w=$(mktemp -d) || exit 2
    status=0
    mkdir -p "$w/bin" "$w/home" "$w/repo/scripts" "$w/repo/src" "$w/repo/examples" \
        "$w/repo/tools/libm-nudge" "$w/viewer/src"
    cp "$scripts/gate.sh" "$scripts/memguard.sh" "$scripts/compare-published.sh" "$w/repo/scripts/"
    printf 'target/\nCargo.lock\n' | tee "$w/repo/.gitignore" > "$w/viewer/.gitignore"
    echo 'pub fn one() {}' > "$w/repo/src/lib.rs"
    echo 'fn main() {}' > "$w/repo/examples/fleetdump.rs"
    echo '[package]' > "$w/repo/tools/libm-nudge/Cargo.toml"
    echo '[package]' | tee "$w/repo/Cargo.toml" > "$w/viewer/Cargo.toml"
    echo '# a lock' > "$w/repo/Cargo.lock"
    echo 'pub fn two() {}' > "$w/viewer/src/lib.rs"
    locked > "$w/viewer/Cargo.lock"
    git -C "$w/repo" init -q
    git -C "$w/viewer" init -q
    cat > "$w/bin/rustc" <<'STUB'
#!/bin/sh
echo rustc stub
STUB
    cat > "$w/bin/cargo" <<'STUB'
#!/bin/sh
args="$*"
echo "$args" >> "$STUB/calls"
if [ -n "${STUB_RULE:-}" ]; then eval "$STUB_RULE"; fi
case "$args" in
    -V) echo cargo stub ;;
    update*)
        awk -v RS= -v ORS='\n\n' '/name = "symbios-avatar"/ { gsub(/\nsource = [^\n]*/, "") } { print }' \
            Cargo.lock > Cargo.lock.new && mv Cargo.lock.new Cargo.lock ;;
    tree*)
        if grep -q '^source' Cargo.lock; then echo 'symbios-avatar v1.0.0'
        else echo 'symbios-avatar v1.0.0 (/somewhere)'; fi ;;
    'test --release --all-features --no-run')
        mkdir -p target/release/deps
        for binary in symbios_avatar-aa integration-bb; do
            printf '#!/bin/sh\n%s\n' 'if [ -n "${STUB_BINARY:-}" ]; then eval "$STUB_BINARY"; fi; echo "test result: ok. 1 passed"' \
                > "target/release/deps/$binary"
            chmod +x "target/release/deps/$binary"
        done
        echo '  Executable unittests src/lib.rs (target/release/deps/symbios_avatar-aa)'
        echo '  Executable tests/main.rs (target/release/deps/integration-bb)' ;;
    *'--example fleetdump'*)
        out=${CARGO_TARGET_DIR:-target}/release/examples
        mkdir -p "$out"
        cp dumpdata "$out/fleet.data"
        printf '#!/bin/sh\ncp "$(dirname "$0")/fleet.data" "$1"\n' > "$out/fleetdump"
        chmod +x "$out/fleetdump" ;;
esac
exit 0
STUB
    chmod +x "$w/bin/rustc" "$w/bin/cargo"
    trap 'rm -rf "$w"' EXIT
}

locked() {
    printf '[[package]]\nname = "symbios-avatar"\nversion = "1.0.0"\nsource = "registry"\n\n'
}

# gate ARGS...: one run in this world, its output kept and its code in $code.
gate() {
    (cd "$w/repo" && exec env HOME="$w/home" PATH="$w/bin:$PATH" STUB="$w" GATE_VIEWER="$w/viewer" \
        GATE_JOBS=2 GATE_MEM_FLOOR=0 scripts/gate.sh "$@") > "$w/out" 2>&1
    code=$?
}
wrong() { echo "  $*"; status=1; }
code_is() { [ "$code" = "$1" ] || wrong "exit $code, not $1: $(tail -3 "$w/out" | tr '\n' ';')"; }
said() { grep -qe "$1" "$w/out" || wrong "did not say: $1"; }
not_said() { grep -qe "$1" "$w/out" && wrong "said: $1"; }
called() { grep -qFxe "$1" "$w/calls" || wrong "never called: $1"; }
count() { grep -cFxe "$1" "$w/calls"; }

first_clippy='clippy --all-targets -- -D warnings'
suite='test --release --all-features --no-fail-fast'

case_runs_every_step_and_ci_matrix() {
    gate
    code_is 0; said '^gate: passed'
    called 'fmt --all -- --check'
    called "$suite"
    called "$first_clippy"
    called 'clippy --all-targets --features serde-avatar -- -D warnings'
    called 'clippy --all-targets --features builtin-clips -- -D warnings'
    called 'clippy --all-targets --all-features -- -D warnings'
    called 'doc --no-deps --document-private-items --all-features'
    called 'doc --no-deps --all-features'
    called 'check --no-default-features'
    called 'check --target wasm32-unknown-unknown --lib --all-features'
}
case_a_repeat_runs_nothing() {
    gate; code_is 0
    gate; code_is 0; said 'tests: passed already'
    [ "$(count "$suite")" = 1 ] || wrong "the suite ran again"
}
case_a_failed_step_fails() {
    STUB_RULE='case "$args" in clippy*builtin-clips*) exit 1 ;; esac' gate
    code_is 1; said '^gate: FAILED'; not_said '^gate: passed'
    STUB_RULE='' gate
    code_is 0
    [ "$(count 'clippy --all-targets --features builtin-clips -- -D warnings')" = 2 ] || wrong "the failed step left a record"
}
case_a_dead_lane_fails() {
    STUB_RULE='[ "$args" = "clippy --all-targets -- -D warnings" ] && kill -9 "$(awk "{print \$4}" /proc/$PPID/stat)"' gate
    code_is 1; said 'docprivate: did not report'; not_said '^gate: passed'
}
case_an_edit_during_a_run_fails_and_keeps_no_record() {
    cp "$w/repo/src/lib.rs" "$w/lib.rs"
    STUB_RULE='case "$args" in clippy*all-features*) echo "// edited" >> src/lib.rs ;; esac' gate
    code_is 1; said 'changed during this run'; not_said '^gate: passed'
    cp "$w/lib.rs" "$w/repo/src/lib.rs"
    STUB_RULE='' gate
    code_is 0
    [ "$(count "$suite")" = 2 ] || wrong "a pass from the edited run was kept"
}
case_an_edit_between_runs_runs_again() {
    gate; code_is 0
    echo '// edited' >> "$w/repo/src/lib.rs"
    gate; code_is 0
    [ "$(count "$suite")" = 2 ] || wrong "the edit went unseen"
}
case_a_step_print_from_outside_keys_nothing() {
    STEP_PRINT=leaked gate; code_is 0
    echo '// edited' >> "$w/repo/src/lib.rs"
    STEP_PRINT=leaked gate; code_is 0
    [ "$(count "$suite")" = 2 ] || wrong "a record was keyed on the caller's variable"
}
case_staging_a_file_moves_nothing() {
    echo new > "$w/repo/src/new.txt"
    gate; code_is 0
    git -C "$w/repo" add src/new.txt
    gate; code_is 0
    [ "$(count "$suite")" = 1 ] || wrong "staging ran the suite again"
}
case_libm_passes_and_is_recorded() {
    gate --libm; code_is 0; said 'libm: nothing new'
    gate --libm; code_is 0; said 'libm: passed already'
}
case_libm_a_known_failure_passes() {
    STUB_BINARY='echo "test anim::turn::tests::a_straight_turn_is_the_walk_that_was_already_there ... FAILED"; echo "test result: FAILED. 0 passed"; exit 101' gate --libm
    code_is 0
}
case_libm_a_new_failure_fails() {
    STUB_BINARY='echo "test some::other::thing ... FAILED"; echo "test result: FAILED. 0 passed"; exit 101' gate --libm
    code_is 1; said 'some::other::thing'; not_said '^gate: passed'
}
case_libm_a_crash_fails() {
    STUB_BINARY='echo "test some::thing ... ok"; exit 139' gate --libm
    code_is 1; not_said '^gate: passed'
}
case_libm_no_summary_fails() {
    STUB_BINARY='exit 0' gate --libm
    code_is 1; not_said '^gate: passed'
}
case_libm_a_missing_binary_fails() {
    STUB_RULE='if [ "$args" = "test --release --all-features --no-run" ]; then echo "  Executable unittests src/lib.rs (target/release/deps/symbios_avatar-aa)"; echo "  Executable tests/main.rs (target/release/deps/integration-bb)"; exit 0; fi' gate --libm
    code_is 1; not_said '^gate: passed'
}
case_libm_a_shim_that_does_not_build_fails() {
    STUB_RULE='[ "$args" = "build --release --quiet" ] && exit 1' gate --libm
    code_is 1; said 'shim'; not_said '^gate: passed'
}
case_libm_builds_what_it_runs_on_a_recorded_suite() {
    gate; code_is 0
    gate --libm; code_is 0
    called 'test --release --all-features --no-run'
    said 'tests: passed already'
}
case_viewer_runs_and_puts_the_lock_back() {
    cp -p "$w/viewer/Cargo.lock" "$w/lock"
    gate --viewer
    code_is 0
    called "test --release --config patch.crates-io.symbios-avatar.path=\"$w/repo\""
    cmp -s "$w/lock" "$w/viewer/Cargo.lock" || wrong "the viewer's lock was not put back"
    [ "$w/viewer/Cargo.lock" -nt "$w/lock" ] && wrong "the viewer's lock has a new time"
}
case_viewer_without_a_checkout_fails() {
    rm -rf "$w/viewer/.git"
    gate --viewer
    code_is 2; not_said '^gate: passed'
}
case_viewer_a_dead_patch_fails() {
    cp -p "$w/viewer/Cargo.lock" "$w/lock"
    STUB_RULE='case "$args" in update*) exit 0 ;; esac' gate --viewer
    code_is 2; said 'patch'; not_said '^gate: passed'
    cmp -s "$w/lock" "$w/viewer/Cargo.lock" || wrong "the viewer's lock was not put back"
}
case_a_stopped_run_stops_its_steps_and_puts_the_lock_back() {
    cp -p "$w/viewer/Cargo.lock" "$w/lock"
    (cd "$w/repo" && exec env HOME="$w/home" PATH="$w/bin:$PATH" STUB="$w" GATE_VIEWER="$w/viewer" \
        GATE_JOBS=2 GATE_MEM_FLOOR=0 \
        STUB_RULE='case "$args" in test*--config*) echo $$ > "$STUB/sleeper"; exec sleep 60 ;; esac' \
        scripts/gate.sh --viewer) > "$w/out" 2>&1 &
    local run=$! tries=0
    until [ -s "$w/sleeper" ] || [ "$tries" = 200 ]; do sleep 0.1; tries=$((tries + 1)); done
    kill -TERM "$run"
    wait "$run"
    code=$?
    code_is 130
    kill -0 "$(cat "$w/sleeper")" 2>/dev/null && wrong "a step outlived the run"
    cmp -s "$w/lock" "$w/viewer/Cargo.lock" || wrong "the viewer's lock was not put back"
    gate --viewer
    code_is 0
}
case_a_killed_runs_lock_goes_back_on_the_next_run() {
    locked > "$w/original"
    mkdir -p "$w/repo/target/gate"
    cp "$w/original" "$w/repo/target/gate/viewer.lock.kept"
    echo patched > "$w/viewer/Cargo.lock"
    gate
    code_is 0
    cmp -s "$w/original" "$w/viewer/Cargo.lock" || wrong "the kept lock was not put back"
}
case_a_second_run_is_refused() {
    mkdir -p "$w/repo/target/gate"
    flock "$w/repo/target/gate/lock" sleep 20 &
    local holder=$!
    sleep 0.3
    gate
    code_is 2; said 'still going'; not_said '^gate: passed'
    kill "$holder" 2>/dev/null
    pkill -P "$holder" 2>/dev/null
}

commit_all() {
    git -C "$w/repo" add -A
    git -C "$w/repo" -c user.name=t -c user.email=t@t -c core.hooksPath=/dev/null \
        -c commit.gpgsign=false commit -qm toy
}
compare() {
    (cd "$w/repo" && exec env HOME="$w/home" PATH="$w/bin:$PATH" STUB="$w" scripts/compare-published.sh) > "$w/out" 2>&1
    code=$?
}
old_dump='person 0 json {"generator":8,"a":1}\nbeast 0 None stamp 8\nrig 1 2 3\n'
case_compare_identical_passes() {
    printf "$old_dump" > "$w/repo/dumpdata"; commit_all
    compare
    code_is 0; said 'verdict: identical'
}
case_compare_ignores_the_generator_stamp() {
    printf "$old_dump" > "$w/repo/dumpdata"; commit_all
    printf 'person 0 json {"generator":9,"a":1}\nbeast 0 None stamp 9\nrig 1 2 3\n' > "$w/repo/dumpdata"
    compare
    code_is 0
}
case_compare_a_moved_row_fails() {
    printf "$old_dump" > "$w/repo/dumpdata"; commit_all
    printf 'person 0 json {"generator":8,"a":1}\nbeast 0 None stamp 8\nrig 1 2 4\n' > "$w/repo/dumpdata"
    compare
    code_is 1; said 'verdict: DIFFERENT'
}
case_compare_one_line_more_fails() {
    printf "$old_dump" > "$w/repo/dumpdata"; commit_all
    printf "${old_dump}rig 9\n" > "$w/repo/dumpdata"
    compare
    code_is 1
}
case_compare_one_line_fewer_fails() {
    printf "$old_dump" > "$w/repo/dumpdata"; commit_all
    printf 'person 0 json {"generator":8,"a":1}\nbeast 0 None stamp 8\n' > "$w/repo/dumpdata"
    compare
    code_is 1
}
case_compare_nothing_read_fails() {
    : > "$w/repo/dumpdata"; commit_all
    compare
    code_is 1; said 'nothing was read'
}

out=$(mktemp -d) || exit 2
trap 'rm -rf "$out"' EXIT
if [ "$#" -gt 0 ]; then
    cases=("$@")
else
    mapfile -t cases < <(declare -F | awk '{print $3}' | grep '^case_')
fi
pids=()
for this_case in "${cases[@]}"; do
    ( declare -F "$this_case" > /dev/null || exit 3; new_world; "$this_case"; exit "$status" ) \
        > "$out/$this_case" 2>&1 &
    pids+=("$!")
done
failed=0
for i in "${!cases[@]}"; do
    if ! wait "${pids[$i]}"; then
        echo "FAILED ${cases[$i]}"
        sed 's/^/ /' "$out/${cases[$i]}"
        failed=$((failed + 1))
    fi
done
echo "gate_test: ${#cases[@]} cases, $failed failed"
[ "$failed" = 0 ]
