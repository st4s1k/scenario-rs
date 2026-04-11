#!/usr/bin/env bash
# run-all.sh — Automated test runner for scenario-rs test scenarios.
#
# Handles the full lifecycle: Docker container, CLI build, test execution.
#
# Usage:
#   ./run-all.sh              # Start Docker, run tests, stop Docker
#   ./run-all.sh --keep       # Start Docker, run tests, leave Docker running
#   ./run-all.sh --no-docker  # Skip Docker management (container must be running)

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Parse flags ---
KEEP_DOCKER=false
NO_DOCKER=false
for arg in "$@"; do
    case "$arg" in
        --keep)      KEEP_DOCKER=true ;;
        --no-docker) NO_DOCKER=true ;;
    esac
done

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# --- Counters ---
passed=0
failed=0
skipped=0

echo -e "${BOLD}=== scenario-rs Test Runner ===${NC}"
echo ""

# --- Docker lifecycle ---
DOCKER_STARTED=false

cleanup_docker() {
    if [[ "$DOCKER_STARTED" == "true" && "$KEEP_DOCKER" == "false" ]]; then
        echo ""
        echo -e "${DIM}Stopping Docker container...${NC}"
        docker compose -f "$SCRIPT_DIR/docker-compose.yml" down --timeout 5 > /dev/null 2>&1
    fi
}
trap cleanup_docker EXIT

if [[ "$NO_DOCKER" == "false" ]]; then
    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Docker not found. Install Docker or use --no-docker.${NC}"
        exit 1
    fi

    # Start container if not already running
    if docker ps --format '{{.Names}}' | grep -q '^test-ssh$'; then
        echo -e "${DIM}Docker container 'test-ssh' already running${NC}"
    else
        echo -e "${DIM}Starting Docker container...${NC}"
        docker compose -f "$SCRIPT_DIR/docker-compose.yml" up -d --wait 2>&1 | \
            sed 's/^/  /' || {
            echo -e "${RED}Failed to start Docker container${NC}"
            exit 1
        }
        DOCKER_STARTED=true

        # Restart so init-dirs.sh sshd_config changes (KEX, PubkeyAuth) take effect
        echo -e "${DIM}Restarting container for sshd config...${NC}"
        docker restart test-ssh > /dev/null 2>&1

        # Wait for sshd to accept connections
        echo -ne "${DIM}Waiting for SSH..."
        for i in $(seq 1 30); do
            if docker exec test-ssh nc -z 127.0.0.1 2222 2>/dev/null; then
                echo -e " ready${NC}"
                break
            fi
            if [[ $i -eq 30 ]]; then
                echo -e " ${RED}timeout${NC}"
                echo -e "${RED}SSH server did not start in time${NC}"
                exit 1
            fi
            sleep 1
            echo -n "."
        done
    fi
fi

# --- Build CLI ---
echo -e "${DIM}Building CLI...${NC}"
(cd "$PROJECT_ROOT" && cargo build -p scenario-rs-cli --quiet)

CLI="$PROJECT_ROOT/target/debug/scenario-rs-cli"
[[ -f "$CLI.exe" ]] && CLI="$CLI.exe"

if [[ ! -f "$CLI" ]]; then
    echo -e "${RED}CLI binary not found at $CLI${NC}"
    exit 1
fi

# --- Upload file for SFTP tests (reuse an existing file) ---
UPLOAD_FILE="$SCRIPT_DIR/README.md"

echo -e "${DIM}Upload file: $UPLOAD_FILE${NC}"
echo ""

# --- Test Runner ---
run_scenario() {
    local config="$1" expect="$2" label="$3"
    shift 3

    printf "  ${CYAN}%-44s${NC} " "$label"

    local output
    output=$("$CLI" --config-path "$SCRIPT_DIR/$config" "$@" 2>&1) || true

    local actual
    if echo "$output" | grep -q "scenario completed successfully"; then
        actual="pass"
    else
        actual="fail"
    fi

    if [[ "$actual" == "$expect" ]]; then
        if [[ "$expect" == "fail" ]]; then
            echo -e "${GREEN}✓ PASS${NC} ${DIM}(expected failure)${NC}"
        else
            echo -e "${GREEN}✓ PASS${NC}"
        fi
        ((passed++)) || true
    else
        echo -e "${RED}✗ FAIL${NC} ${DIM}(expected $expect, got $actual)${NC}"
        echo "$output" | grep -iE "error" | head -5 | sed 's/^/      /'
        ((failed++)) || true
    fi
}

skip_scenario() {
    local label="$1" reason="$2"
    printf "  ${CYAN}%-44s${NC} " "$label"
    echo -e "${YELLOW}⊘ SKIP${NC} ${DIM}($reason)${NC}"
    ((skipped++)) || true
}

# ======================== Password Auth ========================
echo -e "${BOLD}--- Password Auth ---${NC}"

run_scenario "password-auth/empty-steps.toml"                  "pass" "empty-steps"
run_scenario "password-auth/only-sudo-steps.toml"              "pass" "only-sudo-steps"
run_scenario "password-auth/all-succeed.toml"                  "pass" "all-succeed (sftp + sudo)" \
    -r "local_file=$UPLOAD_FILE"
run_scenario "password-auth/sftp-then-sudo.toml"               "pass" "sftp-then-sudo" \
    -r "config_file=$UPLOAD_FILE"
run_scenario "password-auth/only-sftp-steps.toml"              "pass" "only-sftp-steps" \
    -r "app_jar=$UPLOAD_FILE" -r "config_file=$UPLOAD_FILE" -r "scripts_archive=$UPLOAD_FILE"
run_scenario "password-auth/single-sudo-fail.toml"             "fail" "single-sudo-fail"
run_scenario "password-auth/multi-step-mid-fail.toml"          "fail" "multi-step-mid-fail"
run_scenario "password-auth/fail-with-on-fail-succeed.toml"    "fail" "fail-with-on-fail-succeed"
run_scenario "password-auth/fail-with-on-fail-fail.toml"       "fail" "fail-with-on-fail-fail"
run_scenario "password-auth/many-on-fail-steps.toml"           "fail" "many-on-fail-steps"

echo ""

# ======================== Key Auth ========================
echo -e "${BOLD}--- Key Auth ---${NC}"

run_scenario "key-auth/only-sudo-steps.toml"                   "pass" "only-sudo-steps"
run_scenario "key-auth/sftp-then-sudo.toml"                    "pass" "sftp-then-sudo" \
    -r "local_file=$UPLOAD_FILE"

echo ""

# ======================== Agent Auth ========================
echo -e "${BOLD}--- Agent Auth ---${NC}"

# Agent auth requires tokio::net::UnixStream (#[cfg(unix)]) in the compiled
# binary.  The Windows-target binary cannot connect to ssh-agent even when
# running inside Git Bash, so skip on Windows hosts.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        skip_scenario "only-sudo-steps"                        "binary compiled without unix agent support"
        skip_scenario "sftp-then-sudo"                         "binary compiled without unix agent support"
        ;;
    *)
        # Start ssh-agent, load the test key, run scenarios, then clean up.
        AGENT_KEY="$SCRIPT_DIR/key-auth/test_key"
        if [[ -f "$AGENT_KEY" ]]; then
            eval "$(ssh-agent -s)" > /dev/null 2>&1
            chmod 600 "$AGENT_KEY"
            ssh-add "$AGENT_KEY" 2>/dev/null

            run_scenario "agent-auth/only-sudo-steps.toml"             "pass" "only-sudo-steps"
            run_scenario "agent-auth/sftp-then-sudo.toml"              "pass" "sftp-then-sudo" \
                -r "local_file=$UPLOAD_FILE"

            ssh-agent -k > /dev/null 2>&1
        else
            skip_scenario "only-sudo-steps"                            "test key not found"
            skip_scenario "sftp-then-sudo"                             "test key not found"
        fi
        ;;
esac

# ======================== Summary ========================
echo ""
echo -ne "${BOLD}=== Results: "

if [[ $failed -eq 0 ]]; then
    echo -ne "${GREEN}$passed passed${NC}"
else
    echo -ne "${GREEN}$passed passed${NC}${BOLD}, ${RED}$failed failed${NC}"
fi

if [[ $skipped -gt 0 ]]; then
    echo -ne "${BOLD}, ${YELLOW}$skipped skipped${NC}"
fi

echo -e " ${BOLD}===${NC}"

[[ $failed -gt 0 ]] && exit 1
exit 0
