set shell := ["bash", "-uc"]

picea_lab_bind := env_var_or_default("PICEA_LAB_BIND", "127.0.0.1:8080")
picea_lab_api_base := "http://" + picea_lab_bind
picea_lab_web_api_base := env_var_or_default("VITE_PICEA_LAB_API_BASE", picea_lab_api_base)
picea_lab_ready_url := env_var_or_default("PICEA_LAB_READY_URL", picea_lab_api_base + "/api/scenarios")
picea_lab_web_host := env_var_or_default("PICEA_LAB_WEB_HOST", "127.0.0.1")
picea_lab_web_port := env_var_or_default("PICEA_LAB_WEB_PORT", "5173")

alias lab-web := picea-lab-web
alias lab-api := picea-lab-api

default:
    @just --list

# Start the full local workbench: Rust lab API plus the Vite web UI.
picea-lab-web:
    #!/usr/bin/env bash
    set -euo pipefail

    api_bind="$(node crates/picea-lab/web/scripts/resolve-dev-bind.mjs "{{ picea_lab_bind }}")"
    api_base="http://${api_bind}"
    ready_url="${PICEA_LAB_READY_URL:-${api_base}/api/scenarios}"
    web_api_base="${VITE_PICEA_LAB_API_BASE:-${api_base}}"

    echo "Starting picea-lab API on ${api_bind}"
    rtk proxy cargo run -p picea-lab -- serve --bind "${api_bind}" &
    api_pid=$!

    cleanup() {
        kill "$api_pid" 2>/dev/null || true
        wait "$api_pid" 2>/dev/null || true
    }
    trap cleanup EXIT INT TERM

    for attempt in {1..120}; do
        if rtk proxy curl -fsS "${ready_url}" >/dev/null 2>&1; then
            echo "picea-lab API is ready at ${api_base}"
            break
        fi

        if ! kill -0 "$api_pid" 2>/dev/null; then
            wait "$api_pid"
            exit $?
        fi

        if [[ "$attempt" -eq 120 ]]; then
            echo "Timed out waiting for ${ready_url}" >&2
            exit 1
        fi

        sleep 0.5
    done

    echo "Starting picea-lab-web from http://{{ picea_lab_web_host }}:{{ picea_lab_web_port }}"
    # Keep Vite's default non-strict port behavior so local dev can fall through
    # to the next available port when the preferred one is already bound.
    VITE_PICEA_LAB_API_BASE="${web_api_base}" \
        rtk proxy npm --prefix crates/picea-lab/web run dev -- \
        --host "{{ picea_lab_web_host }}" \
        --port "{{ picea_lab_web_port }}"

# Start only the Rust API server used by picea-lab-web.
picea-lab-api:
    api_bind="$(node crates/picea-lab/web/scripts/resolve-dev-bind.mjs "{{ picea_lab_bind }}")"; \
    echo "Starting picea-lab API on ${api_bind}"; \
    rtk proxy cargo run -p picea-lab -- serve --bind "${api_bind}"

# Start only the Vite UI. Set VITE_PICEA_LAB_API_BASE if the API is elsewhere.
picea-lab-web-ui:
    VITE_PICEA_LAB_API_BASE="{{ picea_lab_web_api_base }}" \
        rtk proxy npm --prefix crates/picea-lab/web run dev -- \
        --host "{{ picea_lab_web_host }}" \
        --port "{{ picea_lab_web_port }}"
