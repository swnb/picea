#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
web_root="${repo_root}/crates/picea-lab/web"

run() {
    printf '+'
    printf ' %q' "$@"
    printf '\n'

    if [[ -z "${CI:-}" ]] && command -v rtk >/dev/null 2>&1; then
        rtk proxy "$@"
    else
        "$@"
    fi
}

run_npm() {
    if [[ -z "${CI:-}" ]]; then
        run env \
            "NPM_CONFIG_CACHE=${NPM_CONFIG_CACHE:-${repo_root}/target/npm-cache}" \
            npm "$@"
    else
        run npm "$@"
    fi
}

ensure_web_dependencies() {
    run_npm --prefix "${web_root}" ci
}

run_exact_ignored_test() {
    local test_name="$1"
    local discovered=0
    local expected_line="${test_name}: test"
    local line
    local listed

    if ! listed="$(
        if [[ -z "${CI:-}" ]] && command -v rtk >/dev/null 2>&1; then
            rtk proxy cargo test --locked -p picea-lab --test artifact_run \
                "${test_name}" -- --ignored --exact --list
        else
            cargo test --locked -p picea-lab --test artifact_run \
                "${test_name}" -- --ignored --exact --list
        fi
    )"; then
        printf 'exact ignored test discovery command failed: %s\n' \
            "${test_name}" >&2
        return 1
    fi
    while IFS= read -r line; do
        if [[ "${line}" == "${expected_line}" ]]; then
            discovered=$((discovered + 1))
        fi
    done <<<"${listed}"
    if [[ "${discovered}" -ne 1 ]]; then
        printf 'exact ignored test was not discovered: %s\n' "${test_name}" >&2
        printf '%s\n' "${listed}" >&2
        return 1
    fi

    run cargo test --locked -p picea-lab --test artifact_run \
        "${test_name}" -- --ignored --exact
}

profile_fast() {
    run node --test "${repo_root}/scripts/ci/release-profile.test.mjs"
    run cargo fmt --all -- --check
    run cargo test --locked -p picea --lib
    run cargo test --locked -p picea --examples --no-run

    ensure_web_dependencies
    run_npm --prefix "${web_root}" run test:ui-contract
    run_npm --prefix "${web_root}" run test:i18n
    run_npm --prefix "${web_root}" run test:profile
    run_npm --prefix "${web_root}" run build
}

profile_full() {
    run node --test "${repo_root}/scripts/ci/release-profile.test.mjs"
    run cargo test --locked --workspace --all-targets
    run cargo clippy --locked --workspace --all-targets -- -D warnings

    ensure_web_dependencies
    run_npm --prefix "${web_root}" audit --audit-level=high
    run_npm --prefix "${web_root}" run test:ui-contract
    run_npm --prefix "${web_root}" run test:i18n
    run_npm --prefix "${web_root}" run test:profile
    run_npm --prefix "${web_root}" run build
    run_npm --prefix "${web_root}" run test:dev-server
}

profile_nightly() {
    local failed=0
    local criterion_baseline="picea-ci-current"
    local criterion_started_ms

    run_exact_ignored_test \
        matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed ||
        failed=1
    run_exact_ignored_test \
        matrix_stack_long_run_acceptance_requires_resting_sleep_convergence ||
        failed=1

    criterion_started_ms="$(node -e 'console.log(Date.now())')"
    if run cargo bench --locked -p picea --bench physics_scenarios -- \
        --noplot \
        --sample-size "${PICEA_CRITERION_SAMPLE_SIZE:-20}" \
        --save-baseline "${criterion_baseline}"; then
        run node "${repo_root}/scripts/ci/verify-criterion-counters.mjs" \
            "${criterion_baseline}" "${criterion_started_ms}" ||
            failed=1
    else
        failed=1
    fi

    return "${failed}"
}

profile_release() {
    local git_status
    local metadata_file
    metadata_file="$(mktemp)"

    if [[ -z "${CI:-}" ]] && command -v rtk >/dev/null 2>&1; then
        rtk proxy cargo metadata --locked --no-deps --format-version 1 >"${metadata_file}"
    else
        cargo metadata --locked --no-deps --format-version 1 >"${metadata_file}"
    fi
    run node "${repo_root}/scripts/ci/verify-release-metadata.mjs" "${metadata_file}"
    rm -f "${metadata_file}"

    if [[ -z "${CI:-}" ]] && command -v rtk >/dev/null 2>&1; then
        git_status="$(rtk proxy git status --porcelain)"
    else
        git_status="$(git status --porcelain)"
    fi

    if [[ -n "${CI:-}" && -n "${git_status}" ]]; then
        printf '%s\n' "CI package verification requires a clean checkout" >&2
        printf '%s\n' "${git_status}" >&2
        return 1
    fi

    # Bash 3.2 treats an empty array as unset under nounset. Keep the clean and
    # dirty argv explicit so local macOS and hosted CI enforce the same contract.
    if [[ -n "${git_status}" ]]; then
        printf '%s\n' \
            "Package receipt class: dirty local candidate (buildable, not clean/reproducible)"
        run cargo package -p picea --locked --allow-dirty
        run cargo package -p picea-macro-tools --locked --allow-dirty
    else
        printf '%s\n' "Package receipt class: clean checkout candidate"
        run cargo package -p picea --locked
        run cargo package -p picea-macro-tools --locked
    fi
}

usage() {
    cat <<'EOF'
Usage: scripts/ci/run.sh <profile>

Profiles:
  fast      Runner contract, formatting, core smoke, Web contracts and bundle
  full      Runner contract, workspace tests, strict Clippy and Web contracts
  nightly   Long-window matrix acceptance and Criterion evidence
  release   Package metadata and package build verification (no publish)
EOF
}

cd "${repo_root}"

case "${1:-}" in
    fast)
        profile_fast
        ;;
    full)
        profile_full
        ;;
    nightly)
        profile_nightly
        ;;
    release)
        profile_release
        ;;
    -h | --help | help)
        usage
        ;;
    *)
        usage >&2
        exit 64
        ;;
esac
