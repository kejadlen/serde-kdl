default: all

fmt:
    cargo fmt --all

check:
    cargo check --workspace

clippy:
    cargo clippy --workspace -- -D warnings

coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    export RUSTFLAGS="-Cinstrument-coverage"
    export CARGO_TARGET_DIR="target/coverage"
    export LLVM_PROFILE_FILE="target/coverage/profraw/%p-%m.profraw"
    rm -rf target/coverage
    cargo test --workspace -q
    REPORT=$(grcov target/coverage/profraw \
        --binary-path ./target/coverage/debug/ \
        -s . \
        -t covdir \
        --ignore-not-existing \
        --keep-only 'src/**' \
        --excl-line 'cov-excl-line|unreachable!' \
        --excl-start 'cov-excl-start' \
        --excl-stop 'cov-excl-stop')
    echo "$REPORT" | jq -r '
        def files:
            to_entries[] | .value |
            if .children then .children | files
            else "\(.name): \(.coveragePercent)% (\(.linesCovered)/\(.linesTotal))"
            end;
        .children | files
    '
    COVERAGE=$(echo "$REPORT" | jq '.coveragePercent')
    echo ""
    echo "Total: ${COVERAGE}%"
    if [ "$(echo "$COVERAGE < 100" | bc -l)" -eq 1 ]; then
        echo "ERROR: Coverage is below 100%"
        exit 1
    fi

mutants:
    #!/usr/bin/env bash
    set -uo pipefail
    cargo mutants --timeout-multiplier 3 -j4
    rc=$?
    # 0 = all caught, 3 = timeouts (infinite loops from mutants, still caught)
    if [ "$rc" -eq 0 ] || [ "$rc" -eq 3 ]; then
        exit 0
    fi
    exit "$rc"

release version:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! echo "{{ version }}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
        echo "Usage: just release X.Y.Z[-pre]"
        exit 1
    fi
    if ! jj diff --quiet; then
        echo "Error: working copy has uncommitted changes"
        exit 1
    fi
    sed -i '' 's/^version = ".*"/version = "{{ version }}"/' Cargo.toml
    cargo check --quiet
    jj commit -m 'Release {{ version }}'
    jj bookmark set main -r @-
    jj git push --remote origin -b main
    git -C "$(jj git root)" tag "v{{ version }}" "$(jj log -r @- --no-graph -T 'commit_id')"
    git -C "$(jj git root)" push origin "v{{ version }}"

all: fmt clippy coverage
