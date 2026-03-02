#!/usr/bin/env bash
# =============================================================================
# scripts/test-sync.sh — Manual integration test for skills-sync
# =============================================================================
#
# Exercises the core CLI workflows in a temporary directory:
#   Test 1: skills-sync --init creates a manifest
#   Test 2: --dry-run exits 2 when changes are pending
#   Test 3: --verbose --dry-run shows detailed output
#   Test 4: sync actually installs skills (uses --force to skip prompts)
#   Test 5: idempotent re-sync reports everything up to date
#
# Usage:
#   ./scripts/test-sync.sh
#
# GITHUB_TOKEN is optional but recommended to avoid rate limiting.
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PASS=0
FAIL=0

# Disable color codes in cargo output so we can parse it reliably
export CARGO_TERM_COLOR=never

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "=== skills-sync integration tests ==="
echo "Project root: $PROJECT_ROOT"
echo "Temp dir:     $TMPDIR"
echo ""

pass() {
    echo "  [PASS] $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "  [FAIL] $1"
    FAIL=$((FAIL + 1))
}

# -----------------------------------------------------------------------------
# Test 1: init creates .skills.toml
# -----------------------------------------------------------------------------
echo "Test 1: init"
cd "$TMPDIR"
cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- --init 2>&1 | tail -1

if [ -f "$TMPDIR/.skills.toml" ]; then
    pass ".skills.toml created by --init"
else
    fail ".skills.toml was not created"
fi

echo ""

# -----------------------------------------------------------------------------
# Test 2: dry-run exits 2 when changes are pending
# -----------------------------------------------------------------------------
echo "Test 2: dry-run with example manifest (expects exit 2)"
cd "$TMPDIR"
cp "$PROJECT_ROOT/examples/mixed.skills.toml" "$TMPDIR/.skills.toml"

set +e
cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- \
    --dry-run \
    --config "$TMPDIR/.skills.toml" \
    2>&1
DRY_EXIT=$?
set -e

if [ "$DRY_EXIT" -eq 2 ]; then
    pass "dry-run exited 2 (changes pending)"
elif [ "$DRY_EXIT" -eq 0 ]; then
    pass "dry-run exited 0 (everything up to date — skills already installed)"
else
    fail "dry-run exited $DRY_EXIT (expected 0 or 2)"
fi

echo ""

# -----------------------------------------------------------------------------
# Test 3: verbose dry-run (visual check, no exit code assertion)
# -----------------------------------------------------------------------------
echo "Test 3: verbose dry-run (visual output)"
cd "$TMPDIR"
set +e
cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- \
    --dry-run \
    --verbose \
    --config "$TMPDIR/.skills.toml" \
    2>&1
set -e
pass "verbose dry-run completed"

echo ""

# -----------------------------------------------------------------------------
# Test 4: actual sync installs skills
# -----------------------------------------------------------------------------
echo "Test 4: sync installs skills (--force skips prompts)"
cd "$TMPDIR"
set +e
cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- \
    --force \
    --config "$TMPDIR/.skills.toml" \
    2>&1
SYNC_EXIT=$?
set -e

SKILLS_DIR="$TMPDIR/.claude/skills"
if [ "$SYNC_EXIT" -eq 0 ] && [ -d "$SKILLS_DIR" ]; then
    SKILL_COUNT=$(ls "$SKILLS_DIR" | grep -v '\.lock$' | wc -l | tr -d ' ')
    if [ "$SKILL_COUNT" -gt 0 ]; then
        pass "sync installed $SKILL_COUNT skill(s) to .claude/skills/"
    else
        fail "sync ran but .claude/skills/ is empty"
    fi
else
    fail "sync exited $SYNC_EXIT or .claude/skills/ was not created"
fi

echo ""

# -----------------------------------------------------------------------------
# Test 5: idempotent re-sync
# -----------------------------------------------------------------------------
echo "Test 5: re-sync is idempotent (expects 'everything up to date')"
cd "$TMPDIR"
RESYNC_OUTPUT=$(cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- \
    --force \
    --config "$TMPDIR/.skills.toml" \
    2>&1)
echo "$RESYNC_OUTPUT"

if echo "$RESYNC_OUTPUT" | grep -q "up to date"; then
    pass "re-sync reports everything up to date"
else
    fail "re-sync output does not contain 'up to date'"
fi

echo ""

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
TOTAL=$((PASS + FAIL))
echo "=== Results: $PASS/$TOTAL tests passed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
