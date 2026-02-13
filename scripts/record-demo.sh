#!/usr/bin/env bash
#
# record-demo.sh — Record terminal demos and convert to SVG for README.
#
# Uses asciinema to record in a real PTY (so ANSI colors work natively),
# then svg-term-cli to render crisp animated SVGs.
#
# Prerequisites:
#   brew install asciinema
#   npm install -g svg-term-cli
#
# Usage:
#   ./scripts/record-demo.sh              # Record the main demo
#   ./scripts/record-demo.sh live         # Interactive recording session
#
# Output:
#   assets/demo.svg — Hero image for README

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ASSETS_DIR="$PROJECT_DIR/assets"
BINARY="$PROJECT_DIR/target/release/domain-check"

# SVG settings — clean, no window chrome, no cursor
SVG_WIDTH=90
SVG_PADDING=18

# ── Helpers ──────────────────────────────────────────────────────────────────

die() { echo "ERROR: $*" >&2; exit 1; }

check_deps() {
    command -v asciinema >/dev/null 2>&1 || die "asciinema not found. Install: brew install asciinema"
    command -v svg-term  >/dev/null 2>&1 || die "svg-term not found. Install: npm install -g svg-term-cli"
    [[ -x "$BINARY" ]] || die "Release binary not found. Run: cargo build --release -p domain-check"
}

# Type a string with realistic, variable-speed keystrokes.
# Faster on common chars, micro-pauses on spaces and dashes.
simulate_type() {
    local text="$1"
    for (( i=0; i<${#text}; i++ )); do
        local ch="${text:$i:1}"
        printf '%s' "$ch"
        case "$ch" in
            ' ')  sleep 0.08 ;;   # brief pause on space
            '-')  sleep 0.06 ;;   # slight pause on dash
            ',')  sleep 0.10 ;;   # pause on comma
            *)    sleep 0.035 ;;  # fast base speed
        esac
    done
}

# Simulate typing a command, pause, then execute it.
run_cmd() {
    local cmd_display="$1"   # what to show (e.g. "domain-check google.com")
    local cmd_actual="$2"    # what to actually run (e.g. "$BINARY google.com")

    # Think pause before typing
    sleep 0.6

    # Type the command
    simulate_type "$cmd_display"
    sleep 0.15

    # Press enter and execute
    echo ""
    eval "$cmd_actual"

    # Let the user read the output
    sleep 2.5
}

# Same but with a longer read pause (for bigger output)
run_cmd_long() {
    local cmd_display="$1"
    local cmd_actual="$2"

    sleep 0.6
    simulate_type "$cmd_display"
    sleep 0.15
    echo ""
    eval "$cmd_actual"
    sleep 3.5
}

cast_to_svg() {
    local cast_file="$1"
    local svg_file="$2"

    svg-term \
        --in "$cast_file" \
        --out "$svg_file" \
        --width "$SVG_WIDTH" \
        --padding "$SVG_PADDING" \
        --no-cursor

    local size
    size=$(du -h "$svg_file" | cut -f1)
    echo "  -> $svg_file ($size)"
}

# ── Scenario script ─────────────────────────────────────────────────────────
# This gets executed inside asciinema's PTY so all colors render naturally.

generate_scenario() {
    local scenario_file="$1"
    cat > "$scenario_file" <<'SCENARIO_EOF'
#!/usr/bin/env bash
# This runs inside asciinema's PTY — colors work natively.

BINARY="__BINARY__"

simulate_type() {
    local text="$1"
    for (( i=0; i<${#text}; i++ )); do
        local ch="${text:$i:1}"
        printf '%s' "$ch"
        case "$ch" in
            ' ')  sleep 0.07 ;;
            '-')  sleep 0.05 ;;
            ',')  sleep 0.09 ;;
            *)    sleep 0.03 ;;
        esac
    done
}

# Print a colored comment line to narrate what's happening
comment() {
    printf '\033[1;34m# %s\033[0m\n' "$1"
    sleep 0.8
}

run_cmd() {
    printf '\033[1;36m$\033[0m '
    simulate_type "$1"
    sleep 0.12
    echo ""
    eval "$2"
    sleep 2.5
}

run_cmd_long() {
    printf '\033[1;36m$\033[0m '
    simulate_type "$1"
    sleep 0.12
    echo ""
    eval "$2"
    sleep 3.5
}

# Initial pause
sleep 0.8

# Scene 1: Quick single-domain check
comment "Check a single domain"
run_cmd \
    "domain-check google.com" \
    "$BINARY google.com"

echo ""
sleep 0.3

# Scene 2: Multiple TLDs — fast endpoints, mixed results
comment "Check across multiple TLDs"
run_cmd \
    "domain-check mystartup -t com,org,net,dev --batch" \
    "$BINARY mystartup -t com,org,net,dev --batch"

echo ""
sleep 0.3

# Scene 3: Startup preset — 8 TLDs at once
comment "Use a preset for startup-focused TLDs"
run_cmd \
    "domain-check rustcloud --preset startup --batch" \
    "$BINARY rustcloud --preset startup --batch"

echo ""
sleep 0.3

# Scene 4: Pretty mode — grouped layout, the star feature
comment "Pretty mode: grouped results with sections"
run_cmd_long \
    "domain-check rustcloud --preset startup --pretty --batch" \
    "$BINARY rustcloud --preset startup --pretty --batch"

# Final pause
sleep 1.5
SCENARIO_EOF

    # Inject the actual binary path
    sed -i '' "s|__BINARY__|$BINARY|g" "$scenario_file"
    chmod +x "$scenario_file"
}

# ── Record: Main demo ──────────────────────────────────────────────────────

record_main() {
    echo "Recording main demo..."

    local cast_file="$ASSETS_DIR/demo.cast"
    local svg_file="$ASSETS_DIR/demo.svg"
    local scenario_file="$ASSETS_DIR/.demo-scenario.sh"

    # Generate the scenario
    generate_scenario "$scenario_file"

    # Record with asciinema in a real PTY (v2 format for svg-term compatibility)
    asciinema rec \
        --window-size "${SVG_WIDTH}x35" \
        --overwrite \
        --output-format asciicast-v2 \
        --command "$scenario_file" \
        "$cast_file"

    # Clean up scenario
    rm -f "$scenario_file"

    # Convert to SVG
    cast_to_svg "$cast_file" "$svg_file"

    # Clean up cast file (optional — keep for re-rendering)
    echo ""
    echo "Main demo recorded successfully."
    echo "  Cast: $cast_file (keep this to re-render with different settings)"
    echo "  SVG:  $svg_file"
}

# ── Record: Live (interactive) ──────────────────────────────────────────────

record_live() {
    echo "Starting live recording session..."
    echo "The release binary is on your PATH. Type 'domain-check' directly."
    echo "Press Ctrl-D or type 'exit' when done."
    echo ""

    local cast_file="$ASSETS_DIR/demo-live.cast"
    local svg_file="$ASSETS_DIR/demo-live.svg"

    export PATH="$(dirname "$BINARY"):$PATH"

    asciinema rec \
        --window-size "${SVG_WIDTH}x35" \
        --overwrite \
        --output-format asciicast-v2 \
        "$cast_file"

    cast_to_svg "$cast_file" "$svg_file"
    echo "Live demo done."
}

# ── Re-render: Convert existing cast to SVG ─────────────────────────────────

rerender() {
    local cast_file="${1:-$ASSETS_DIR/demo.cast}"
    local svg_file="${cast_file%.cast}.svg"

    [[ -f "$cast_file" ]] || die "Cast file not found: $cast_file"

    echo "Re-rendering $cast_file..."
    cast_to_svg "$cast_file" "$svg_file"
}

# ── Main ────────────────────────────────────────────────────────────────────

main() {
    check_deps
    mkdir -p "$ASSETS_DIR"

    local target="${1:-main}"

    case "$target" in
        main)     record_main ;;
        live)     record_live ;;
        rerender) rerender "${2:-}" ;;
        *)
            echo "Usage: $0 [main|live|rerender [cast-file]]"
            echo ""
            echo "  main      Record scripted demo (default mode + pretty mode)"
            echo "  live      Interactive recording session"
            echo "  rerender  Re-render existing .cast file to SVG"
            exit 1
            ;;
    esac
}

main "$@"
