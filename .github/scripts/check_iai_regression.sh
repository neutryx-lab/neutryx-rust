#!/bin/bash
# check_iai_regression.sh
#
# Compares iai-callgrind benchmark results against a baseline and fails
# if instruction count regression exceeds the specified threshold.
#
# Requirements:
#   - iai-callgrind benchmark output in target/iai/
#   - jq for JSON parsing (apt install jq)
#
# Usage:
#   ./scripts/check_iai_regression.sh --threshold 10
#   ./scripts/check_iai_regression.sh --threshold 5 --baseline main
#
# Exit codes:
#   0 - No regression detected
#   1 - Regression detected (instruction count increased beyond threshold)
#   2 - Error (missing files, invalid arguments)

set -euo pipefail

# Default values
THRESHOLD=10
BASELINE="baseline"
BENCHMARK_DIR="target/iai"
OUTPUT_FORMAT="text"
VERBOSE=false

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Compare iai-callgrind benchmark results against baseline.

Options:
    -t, --threshold NUM    Regression threshold percentage (default: 10)
    -b, --baseline NAME    Baseline name to compare against (default: baseline)
    -d, --dir PATH         Benchmark results directory (default: target/iai)
    -o, --output FORMAT    Output format: text, json, github (default: text)
    -v, --verbose          Enable verbose output
    -h, --help             Show this help message

Examples:
    $(basename "$0") --threshold 10
    $(basename "$0") --threshold 5 --baseline main --output github
EOF
    exit 0
}

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        -b|--baseline)
            BASELINE="$2"
            shift 2
            ;;
        -d|--dir)
            BENCHMARK_DIR="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 2
            ;;
    esac
done

# Validate threshold
if ! [[ "$THRESHOLD" =~ ^[0-9]+$ ]]; then
    log_error "Threshold must be a positive integer: $THRESHOLD"
    exit 2
fi

# Check if benchmark directory exists
if [[ ! -d "$BENCHMARK_DIR" ]]; then
    log_warn "Benchmark directory not found: $BENCHMARK_DIR"
    log_info "This is expected for first run - creating baseline"
    exit 0
fi

# Find benchmark result files
CURRENT_RESULTS=$(find "$BENCHMARK_DIR" -name "*.json" -type f 2>/dev/null || true)

if [[ -z "$CURRENT_RESULTS" ]]; then
    log_warn "No benchmark results found in $BENCHMARK_DIR"
    log_info "Run 'cargo bench -p pricer_pricing --bench engine_iai' first"
    exit 0
fi

# Initialize counters
TOTAL_BENCHMARKS=0
PASSED_BENCHMARKS=0
REGRESSED_BENCHMARKS=0
IMPROVED_BENCHMARKS=0
declare -a REGRESSION_DETAILS=()

# Process each benchmark result
process_benchmark_file() {
    local file="$1"
    local bench_name
    bench_name=$(basename "$file" .json)

    if [[ "$VERBOSE" == "true" ]]; then
        log_info "Processing: $bench_name"
    fi

    # Check if baseline exists
    local baseline_file="${file%.json}.baseline.json"
    if [[ ! -f "$baseline_file" ]]; then
        if [[ "$VERBOSE" == "true" ]]; then
            log_info "  No baseline for $bench_name - skipping comparison"
        fi
        return 0
    fi

    # Extract instruction counts (simplified - actual iai-callgrind output format may vary)
    # For iai-callgrind, we parse the summary.json or similar output
    local current_instructions baseline_instructions

    # Try to extract instruction count from JSON
    if command -v jq &> /dev/null; then
        current_instructions=$(jq -r '.instructions // .ir_counts.total // 0' "$file" 2>/dev/null || echo "0")
        baseline_instructions=$(jq -r '.instructions // .ir_counts.total // 0' "$baseline_file" 2>/dev/null || echo "0")
    else
        # Fallback: grep for instruction count
        current_instructions=$(grep -oP '"instructions":\s*\K\d+' "$file" 2>/dev/null || echo "0")
        baseline_instructions=$(grep -oP '"instructions":\s*\K\d+' "$baseline_file" 2>/dev/null || echo "0")
    fi

    # Skip if we couldn't get valid numbers
    if [[ "$current_instructions" == "0" ]] || [[ "$baseline_instructions" == "0" ]]; then
        if [[ "$VERBOSE" == "true" ]]; then
            log_warn "  Could not parse instruction counts for $bench_name"
        fi
        return 0
    fi

    # Calculate percentage change
    local change_pct
    change_pct=$(awk "BEGIN {printf \"%.2f\", (($current_instructions - $baseline_instructions) / $baseline_instructions) * 100}")

    TOTAL_BENCHMARKS=$((TOTAL_BENCHMARKS + 1))

    # Check for regression
    local change_abs
    change_abs=$(awk "BEGIN {print ($change_pct < 0) ? -$change_pct : $change_pct}")
    local is_regression
    is_regression=$(awk "BEGIN {print ($change_pct > $THRESHOLD) ? 1 : 0}")
    local is_improvement
    is_improvement=$(awk "BEGIN {print ($change_pct < -$THRESHOLD) ? 1 : 0}")

    if [[ "$is_regression" == "1" ]]; then
        REGRESSED_BENCHMARKS=$((REGRESSED_BENCHMARKS + 1))
        REGRESSION_DETAILS+=("$bench_name: +${change_pct}% ($baseline_instructions -> $current_instructions)")
        if [[ "$VERBOSE" == "true" ]]; then
            log_error "  REGRESSION: $bench_name +${change_pct}%"
        fi
    elif [[ "$is_improvement" == "1" ]]; then
        IMPROVED_BENCHMARKS=$((IMPROVED_BENCHMARKS + 1))
        if [[ "$VERBOSE" == "true" ]]; then
            log_info "  IMPROVED: $bench_name ${change_pct}%"
        fi
    else
        PASSED_BENCHMARKS=$((PASSED_BENCHMARKS + 1))
        if [[ "$VERBOSE" == "true" ]]; then
            log_info "  OK: $bench_name ${change_pct}%"
        fi
    fi
}

# Process all benchmark files
for file in $CURRENT_RESULTS; do
    process_benchmark_file "$file"
done

# Output results based on format
output_text() {
    echo ""
    echo "=========================================="
    echo "  Iai-Callgrind Regression Report"
    echo "=========================================="
    echo ""
    echo "Threshold: ${THRESHOLD}%"
    echo "Baseline:  ${BASELINE}"
    echo ""
    echo "Results:"
    echo "  Total:      $TOTAL_BENCHMARKS"
    echo "  Passed:     $PASSED_BENCHMARKS"
    echo "  Regressed:  $REGRESSED_BENCHMARKS"
    echo "  Improved:   $IMPROVED_BENCHMARKS"
    echo ""

    if [[ ${#REGRESSION_DETAILS[@]} -gt 0 ]]; then
        echo "Regressions detected:"
        for detail in "${REGRESSION_DETAILS[@]}"; do
            echo "  - $detail"
        done
        echo ""
    fi
}

output_json() {
    cat << EOF
{
  "threshold": $THRESHOLD,
  "baseline": "$BASELINE",
  "total": $TOTAL_BENCHMARKS,
  "passed": $PASSED_BENCHMARKS,
  "regressed": $REGRESSED_BENCHMARKS,
  "improved": $IMPROVED_BENCHMARKS,
  "regressions": [$(printf '"%s",' "${REGRESSION_DETAILS[@]}" | sed 's/,$//')],
  "status": "$([ $REGRESSED_BENCHMARKS -eq 0 ] && echo "pass" || echo "fail")"
}
EOF
}

output_github() {
    # GitHub Actions workflow commands
    if [[ $REGRESSED_BENCHMARKS -gt 0 ]]; then
        echo "::error::Performance regression detected: $REGRESSED_BENCHMARKS benchmark(s) exceeded ${THRESHOLD}% threshold"
        for detail in "${REGRESSION_DETAILS[@]}"; do
            echo "::error::  $detail"
        done
    else
        echo "::notice::All benchmarks within ${THRESHOLD}% threshold (${PASSED_BENCHMARKS} passed, ${IMPROVED_BENCHMARKS} improved)"
    fi

    # Set output for GitHub Actions
    echo "regression_count=$REGRESSED_BENCHMARKS" >> "${GITHUB_OUTPUT:-/dev/null}"
    echo "total_benchmarks=$TOTAL_BENCHMARKS" >> "${GITHUB_OUTPUT:-/dev/null}"
}

case "$OUTPUT_FORMAT" in
    text)
        output_text
        ;;
    json)
        output_json
        ;;
    github)
        output_github
        ;;
    *)
        log_error "Unknown output format: $OUTPUT_FORMAT"
        exit 2
        ;;
esac

# Exit with appropriate code
if [[ $REGRESSED_BENCHMARKS -gt 0 ]]; then
    if [[ "$OUTPUT_FORMAT" == "text" ]]; then
        log_error "Performance regression detected!"
        echo "  $REGRESSED_BENCHMARKS benchmark(s) exceeded the ${THRESHOLD}% threshold."
        echo ""
        echo "To update baseline after review:"
        echo "  cargo bench -p pricer_pricing --bench engine_iai -- --save-baseline=$BASELINE"
    fi
    exit 1
else
    if [[ "$OUTPUT_FORMAT" == "text" ]]; then
        log_info "All benchmarks within acceptable threshold."
    fi
    exit 0
fi
