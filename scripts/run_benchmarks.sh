#!/bin/bash
#
# Hybrid Benchmark Execution with Variance Control
#
# Strategy: Sequential groups, parallel indicators within groups
# - Reduces CPU contention vs full parallel (13 cores)
# - Better than pure sequential (no parallelism)
# - Multi-round execution with cooldown for thermal stability
#
# Expected performance:
# - Speedup: 3-5x vs sequential
# - Variance: Low (manageable with median/MAD aggregation)
# - Time: 5-7 minutes per round
#
# Based on analysis in /tmp/sample_size_analysis.md

set -euo pipefail

# Configuration
ROUNDS=${ROUNDS:-3}                    # Number of benchmark rounds
COOLDOWN=${COOLDOWN:-10}               # Cooldown between groups (seconds)
BENCHMARK_BIN="talib_comparison"       # Benchmark suite to run
RESULTS_DIR="target/criterion"         # Criterion output directory
BASELINE_PREFIX="round"                # Baseline name prefix

# Benchmark groups (organized by computational similarity to minimize variance)
# Format: "group_name:indicator1,indicator2,..."
BENCHMARK_GROUPS=(
    "moving_averages:sma,ema,wma,dema,tema,trima"
    "momentum:rsi,roc,mom,cmo,apo,trix"
    "volatility:atr,trange,bollinger"
    "trend:adx,dx,aroon"
    "oscillators:cci,mfi,williams_r,bop,ultosc"
    "stochastic:stochastic,stochastic_fast"
    "volume_price:ad,obv,midpoint,midprice"
    "advanced:var,tsf,linearreg,kama,t3,macd"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Print configuration
print_config() {
    cat << EOF

========================================
Benchmark Configuration
========================================
Rounds:          $ROUNDS
Cooldown:        ${COOLDOWN}s (between groups)
Groups:          ${#BENCHMARK_GROUPS[@]}
Benchmark Suite: $BENCHMARK_BIN
Results Dir:     $RESULTS_DIR

Estimated time per round: $((${#BENCHMARK_GROUPS[@]} * COOLDOWN / 60 + 5)) minutes
Total estimated time:     $(($ROUNDS * (${#BENCHMARK_GROUPS[@]} * COOLDOWN / 60 + 5))) minutes
========================================

EOF
}

# Wait for CPU cooldown
wait_cooldown() {
    local group_name=$1

    log_info "Cooling down for ${COOLDOWN}s after group: $group_name"

    # Show progress bar
    for ((i=1; i<=COOLDOWN; i++)); do
        printf "\r[%-50s] %d/%d seconds" \
            "$(printf '#%.0s' $(seq 1 $((i * 50 / COOLDOWN))))" \
            "$i" "$COOLDOWN"
        sleep 1
    done
    echo ""

    log_info "Cooldown complete"
}

# Find the benchmark binary
find_benchmark_binary() {
    # Find the most recent benchmark binary (macOS-compatible)
    local binary_path=$(ls -t target/release/deps/${BENCHMARK_BIN}-* 2>/dev/null | grep -v '\.d$' | head -1)
    if [ -z "$binary_path" ]; then
        log_error "Benchmark binary not found: ${BENCHMARK_BIN}"
        return 1
    fi
    if [ ! -x "$binary_path" ]; then
        log_error "Benchmark binary not executable: $binary_path"
        return 1
    fi
    echo "$binary_path"
}

# Run a single benchmark group in parallel
run_group() {
    local round=$1
    local group_spec=$2
    local benchmark_binary=$3
    local group_name="${group_spec%%:*}"
    local indicators="${group_spec#*:}"

    log_info "Round $round - Group: $group_name"
    log_info "Indicators: $indicators"

    local baseline_name="${BASELINE_PREFIX}${round}_${group_name}"
    log_info "Running with baseline: $baseline_name"

    # Convert comma-separated list to array
    IFS=',' read -ra INDICATOR_ARRAY <<< "$indicators"

    # Run each indicator as a background job for parallel execution
    # Use pre-built binary directly to avoid Cargo lock contention
    local pids=()
    for indicator in "${INDICATOR_ARRAY[@]}"; do
        (
            log_info "  Starting: $indicator (warmup 5s, measurement 10s, 500 samples)"
            # Run benchmark with explicit bench mode and save baseline
            if "$benchmark_binary" \
                --bench \
                "^$indicator\$" \
                --save-baseline "$baseline_name" \
                2>&1 | grep -E "Benchmarking|time:|found" | sed "s/^/    [$indicator] /"; then
                log_success "  Completed: $indicator"
            else
                log_error "  Failed: $indicator"
                exit 1
            fi
        ) &
        pids+=($!)
    done

    # Wait for all background jobs to complete
    local failed=0
    for pid in "${pids[@]}"; do
        if ! wait "$pid"; then
            failed=1
        fi
    done

    if [ $failed -eq 1 ]; then
        log_error "Group $group_name had failures"
        return 1
    fi

    log_success "Group $group_name completed"
}

# Run all rounds
run_all_rounds() {
    local start_time=$(date +%s)

    # Find the benchmark binary once
    log_info "Locating benchmark binary..."
    local benchmark_binary
    if ! benchmark_binary=$(find_benchmark_binary); then
        log_error "Failed to locate benchmark binary"
        return 1
    fi
    log_info "Using binary: $benchmark_binary"

    for round in $(seq 1 "$ROUNDS"); do
        log_info "========================================="
        log_info "Starting Round $round/$ROUNDS"
        log_info "========================================="

        local round_start=$(date +%s)

        local group_count=0
        local total_groups=${#BENCHMARK_GROUPS[@]}

        for group_spec in "${BENCHMARK_GROUPS[@]}"; do
            group_count=$((group_count + 1))
            local group_name="${group_spec%%:*}"

            # Run the group with pre-built binary
            if ! run_group "$round" "$group_spec" "$benchmark_binary"; then
                log_error "Group $group_name failed in round $round"
                return 1
            fi

            # Cooldown between groups (but not after the last group)
            if [ $group_count -lt $total_groups ]; then
                wait_cooldown "$group_name"
            fi
        done

        local round_end=$(date +%s)
        local round_duration=$((round_end - round_start))

        log_success "Round $round completed in $((round_duration / 60))m $((round_duration % 60))s"

        # Longer cooldown between rounds (if not last round)
        if [ "$round" -lt "$ROUNDS" ]; then
            log_info "Inter-round cooldown: 120s"
            sleep 120
        fi
    done

    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))

    log_success "========================================="
    log_success "All rounds completed!"
    log_success "Total time: $((total_duration / 60))m $((total_duration % 60))s"
    log_success "========================================="
}

# Aggregate results
aggregate_results() {
    log_info "Aggregating results across rounds..."

    if [ -f "scripts/aggregate_benchmarks.py" ]; then
        python3 scripts/aggregate_benchmarks.py \
            --results-dir "$RESULTS_DIR" \
            --rounds "$ROUNDS" \
            --baseline-prefix "$BASELINE_PREFIX"

        log_success "Aggregation complete. See target/criterion/aggregated/"
    else
        log_warning "Aggregation script not found. Skipping aggregation."
        log_info "Run manually: python3 scripts/aggregate_benchmarks.py"
    fi
}

# Main execution
main() {
    print_config

    # Check dependencies
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        exit 1
    fi

    # Optional: Check for GNU parallel or note we're using xargs
    log_info "Using shell job control for parallel execution"

    # Build benchmarks first
    log_info "Building benchmarks..."
    if ! cargo bench --bench "$BENCHMARK_BIN" --no-run; then
        log_error "Benchmark build failed"
        exit 1
    fi
    log_success "Benchmarks built successfully"

    # Run all rounds
    if ! run_all_rounds; then
        log_error "Benchmark execution failed"
        exit 1
    fi

    # Aggregate results
    aggregate_results

    log_success "Benchmark run complete!"
    log_info "Results saved to: $RESULTS_DIR"
}

# Run main function
main "$@"
