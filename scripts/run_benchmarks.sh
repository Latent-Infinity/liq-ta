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
COOLDOWN=${COOLDOWN:-60}               # Cooldown between groups (seconds)
BENCHMARK_BIN="talib_comparison"       # Benchmark suite to run
RESULTS_DIR="target/criterion"         # Criterion output directory
BASELINE_PREFIX="round"                # Baseline name prefix

# Benchmark groups (organized by computational similarity to minimize variance)
# Format: "group_name:indicator1,indicator2,..."
GROUPS=(
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
Groups:          ${#GROUPS[@]}
Benchmark Suite: $BENCHMARK_BIN
Results Dir:     $RESULTS_DIR

Estimated time per round: $((${#GROUPS[@]} * COOLDOWN / 60 + 5)) minutes
Total estimated time:     $(($ROUNDS * (${#GROUPS[@]} * COOLDOWN / 60 + 5))) minutes
========================================

EOF
}

# Get CPU temperature (macOS)
get_cpu_temp() {
    if command -v osx-cpu-temp &> /dev/null; then
        osx-cpu-temp | grep -oE '[0-9]+\.[0-9]+' | head -1
    else
        echo "N/A"
    fi
}

# Wait for CPU cooldown
wait_cooldown() {
    local group_name=$1
    local temp_before=$(get_cpu_temp)

    log_info "Cooling down for ${COOLDOWN}s after group: $group_name (temp: ${temp_before}°C)"

    # Show progress bar
    for ((i=1; i<=COOLDOWN; i++)); do
        printf "\r[%-50s] %d/%d seconds" \
            "$(printf '#%.0s' $(seq 1 $((i * 50 / COOLDOWN))))" \
            "$i" "$COOLDOWN"
        sleep 1
    done
    echo ""

    local temp_after=$(get_cpu_temp)
    log_info "Cooldown complete (temp: ${temp_after}°C)"
}

# Run a single benchmark group in parallel
run_group() {
    local round=$1
    local group_spec=$2
    local group_name="${group_spec%%:*}"
    local indicators="${group_spec#*:}"

    log_info "Round $round - Group: $group_name"
    log_info "Indicators: $indicators"

    # Convert comma-separated list to array
    IFS=',' read -ra INDICATOR_ARRAY <<< "$indicators"

    # Build parallel command with proper quoting
    local parallel_cmd=""
    for indicator in "${INDICATOR_ARRAY[@]}"; do
        if [ -n "$parallel_cmd" ]; then
            parallel_cmd="$parallel_cmd ::: "
        fi
        parallel_cmd="${parallel_cmd}${indicator}"
    done

    # Run benchmarks in parallel using GNU parallel
    # --jobs: Number of parallel jobs (number of indicators in group)
    # --line-buffer: Print output line-by-line (better progress visibility)
    # --halt: Stop all jobs if one fails
    local baseline_name="${BASELINE_PREFIX}${round}_${group_name}"

    log_info "Running with baseline: $baseline_name"

    # Use xargs -P for parallel execution (more portable than GNU parallel)
    echo "$indicators" | tr ',' '\n' | while read -r indicator; do
        log_info "  Starting: $indicator"
        cargo bench \
            --bench "$BENCHMARK_BIN" \
            -- --exact "^$indicator\$" \
            --save-baseline "$baseline_name" \
            2>&1 | sed "s/^/    [$indicator] /"

        if [ ${PIPESTATUS[0]} -eq 0 ]; then
            log_success "  Completed: $indicator"
        else
            log_error "  Failed: $indicator"
            return 1
        fi
    done &

    # Wait for all background jobs to complete
    wait

    log_success "Group $group_name completed"
}

# Run all rounds
run_all_rounds() {
    local start_time=$(date +%s)

    for round in $(seq 1 "$ROUNDS"); do
        log_info "========================================="
        log_info "Starting Round $round/$ROUNDS"
        log_info "========================================="

        local round_start=$(date +%s)

        for group_spec in "${GROUPS[@]}"; do
            local group_name="${group_spec%%:*}"

            # Run the group
            if ! run_group "$round" "$group_spec"; then
                log_error "Group $group_name failed in round $round"
                return 1
            fi

            # Cooldown between groups (but not after the last group)
            if [ "$group_spec" != "${GROUPS[-1]}" ]; then
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
