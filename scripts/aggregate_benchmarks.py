#!/usr/bin/env python3
"""
Aggregate benchmark results across multiple rounds using robust statistics.

This script:
1. Reads Criterion JSON results from multiple benchmark rounds
2. Computes median and MAD (Median Absolute Deviation) across rounds
3. Detects outliers and high variance using coefficient of variation
4. Generates aggregated report with quality metrics

Based on variance management strategy from /tmp/sample_size_analysis.md
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import statistics


class BenchmarkAggregator:
    """Aggregates benchmark results using robust statistics."""

    def __init__(self, results_dir: Path, rounds: int, baseline_prefix: str):
        self.results_dir = Path(results_dir)
        self.rounds = rounds
        self.baseline_prefix = baseline_prefix
        self.results = {}

    def collect_results(self) -> Dict[str, List[float]]:
        """Collect median times from all rounds for each benchmark.

        Returns:
            Dict mapping benchmark name to list of median times (ns) across rounds.
        """
        benchmark_times = {}

        # Iterate through all benchmark directories
        if not self.results_dir.exists():
            print(f"Error: Results directory not found: {self.results_dir}", file=sys.stderr)
            return {}

        for benchmark_dir in self.results_dir.iterdir():
            if not benchmark_dir.is_dir() or benchmark_dir.name.startswith('.'):
                continue

            benchmark_name = benchmark_dir.name
            times = []

            # Criterion nests results: benchmark/implementation/size/baseline/base/estimates.json
            # We need to search recursively for baseline directories
            for round_num in range(1, self.rounds + 1):
                # Try different group names (we don't know which group this benchmark was in)
                # Include both old group names (8 groups) and new group names (3 groups)
                for group_name in ["fast_indicators", "simple_volume", "complex_indicators",
                                   "moving_averages", "momentum", "volatility", "trend",
                                   "oscillators", "stochastic", "volume_price", "advanced"]:
                    baseline_name = f"{self.baseline_prefix}{round_num}_{group_name}"

                    # Search for baseline directories recursively (handles nested structure)
                    baseline_dirs = list(benchmark_dir.glob(f"**/{baseline_name}"))

                    if baseline_dirs:
                        # Take the first match (usually liq-ta implementation)
                        # Could aggregate both liq-ta and ta-lib if needed
                        estimates_file = baseline_dirs[0] / "estimates.json"

                        if estimates_file.exists():
                            try:
                                with open(estimates_file) as f:
                                    data = json.load(f)
                                    # Extract median point estimate (in nanoseconds)
                                    median_ns = data["median"]["point_estimate"]
                                    times.append(median_ns)
                                    break  # Found this round's result
                            except (json.JSONDecodeError, KeyError) as e:
                                print(f"Warning: Failed to parse {estimates_file}: {e}", file=sys.stderr)

            if times:
                benchmark_times[benchmark_name] = times
            else:
                # Check if this benchmark has any baseline directories matching our pattern
                # If not, it's from a different benchmark run and we should skip it silently
                has_matching_baseline = False
                for round_num in range(1, self.rounds + 1):
                    pattern = f"**/{self.baseline_prefix}{round_num}_*"
                    if list(benchmark_dir.glob(pattern)):
                        has_matching_baseline = True
                        break

                # Only warn if it has matching baselines but incomplete rounds
                if has_matching_baseline:
                    print(f"Warning: Incomplete results for benchmark: {benchmark_name} ({len(times)}/{self.rounds} rounds)", file=sys.stderr)

        return benchmark_times

    @staticmethod
    def median_absolute_deviation(values: List[float]) -> float:
        """Compute MAD (Median Absolute Deviation).

        MAD = median(|x_i - median(x)|)

        More robust than standard deviation for outlier detection.
        """
        if not values:
            return 0.0

        median = statistics.median(values)
        deviations = [abs(x - median) for x in values]
        return statistics.median(deviations)

    @staticmethod
    def coefficient_of_variation(values: List[float]) -> float:
        """Compute Coefficient of Variation (CV).

        CV = (MAD / median) * 100%

        Normalized measure of dispersion. Lower is better.
        - CV < 5%: Excellent stability
        - CV 5-10%: Good stability
        - CV 10-20%: Acceptable
        - CV > 20%: Poor (investigate thermal/contention issues)
        """
        if not values:
            return 0.0

        median = statistics.median(values)
        if median == 0:
            return 0.0

        mad = BenchmarkAggregator.median_absolute_deviation(values)
        return (mad / median) * 100.0

    @staticmethod
    def detect_outliers_iqr(values: List[float]) -> Tuple[List[float], List[float]]:
        """Detect outliers using IQR (Interquartile Range) method.

        Outlier if: x < Q1 - 1.5*IQR  or  x > Q3 + 1.5*IQR

        Returns:
            Tuple of (clean_values, outliers)
        """
        if len(values) < 4:
            return values, []

        q1 = statistics.quantiles(values, n=4)[0]  # 25th percentile
        q3 = statistics.quantiles(values, n=4)[2]  # 75th percentile
        iqr = q3 - q1

        lower_bound = q1 - 1.5 * iqr
        upper_bound = q3 + 1.5 * iqr

        clean = [x for x in values if lower_bound <= x <= upper_bound]
        outliers = [x for x in values if x < lower_bound or x > upper_bound]

        return clean, outliers

    def format_time(self, ns: float) -> str:
        """Format nanoseconds in human-readable units."""
        if ns < 1_000:
            return f"{ns:.2f} ns"
        elif ns < 1_000_000:
            return f"{ns / 1_000:.2f} µs"
        elif ns < 1_000_000_000:
            return f"{ns / 1_000_000:.2f} ms"
        else:
            return f"{ns / 1_000_000_000:.2f} s"

    def generate_report(self, benchmark_times: Dict[str, List[float]]) -> str:
        """Generate aggregated report with quality metrics.

        Returns:
            Formatted report as string.
        """
        if not benchmark_times:
            return "No benchmark results found."

        lines = []
        lines.append("=" * 100)
        lines.append("BENCHMARK AGGREGATION REPORT")
        lines.append("=" * 100)
        lines.append(f"Rounds: {self.rounds}")
        lines.append(f"Benchmarks: {len(benchmark_times)}")
        lines.append("")
        lines.append("Aggregation method: Median (robust central tendency)")
        lines.append("Variance metric: MAD (Median Absolute Deviation)")
        lines.append("Quality metric: CV (Coefficient of Variation = MAD/Median * 100%)")
        lines.append("")
        lines.append("Quality thresholds:")
        lines.append("  CV < 5%:   Excellent ✓")
        lines.append("  CV 5-10%:  Good")
        lines.append("  CV 10-20%: Acceptable")
        lines.append("  CV > 20%:  Poor (investigate)")
        lines.append("=" * 100)
        lines.append("")

        # Sort benchmarks by name
        sorted_benchmarks = sorted(benchmark_times.items())

        # Compute statistics
        results = []
        for benchmark_name, times in sorted_benchmarks:
            if len(times) < self.rounds:
                print(f"Warning: {benchmark_name} has only {len(times)}/{self.rounds} rounds",
                      file=sys.stderr)

            median = statistics.median(times)
            mad = self.median_absolute_deviation(times)
            cv = self.coefficient_of_variation(times)
            clean_times, outliers = self.detect_outliers_iqr(times)

            results.append({
                "name": benchmark_name,
                "median": median,
                "mad": mad,
                "cv": cv,
                "rounds": len(times),
                "outliers": len(outliers),
                "times": times,
            })

        # Print table header
        lines.append(f"{'Benchmark':<30} {'Median':>12} {'MAD':>12} {'CV':>8} {'Rounds':>7} {'Outliers':>9} {'Quality':>10}")
        lines.append("-" * 100)

        # Print results
        for r in results:
            quality = "✓ Excellent" if r["cv"] < 5 else \
                     "Good" if r["cv"] < 10 else \
                     "Acceptable" if r["cv"] < 20 else \
                     "⚠ Poor"

            lines.append(
                f"{r['name']:<30} "
                f"{self.format_time(r['median']):>12} "
                f"{self.format_time(r['mad']):>12} "
                f"{r['cv']:>7.2f}% "
                f"{r['rounds']:>7} "
                f"{r['outliers']:>9} "
                f"{quality:>10}"
            )

        lines.append("")
        lines.append("=" * 100)

        # Summary statistics
        cv_values = [r["cv"] for r in results]
        excellent = sum(1 for cv in cv_values if cv < 5)
        good = sum(1 for cv in cv_values if 5 <= cv < 10)
        acceptable = sum(1 for cv in cv_values if 10 <= cv < 20)
        poor = sum(1 for cv in cv_values if cv >= 20)

        lines.append("SUMMARY")
        lines.append("-" * 100)
        lines.append(f"Total benchmarks: {len(results)}")
        lines.append(f"  Excellent (CV < 5%):   {excellent:3} ({excellent / len(results) * 100:.1f}%)")
        lines.append(f"  Good (CV 5-10%):       {good:3} ({good / len(results) * 100:.1f}%)")
        lines.append(f"  Acceptable (CV 10-20%): {acceptable:3} ({acceptable / len(results) * 100:.1f}%)")
        lines.append(f"  Poor (CV > 20%):       {poor:3} ({poor / len(results) * 100:.1f}%)")
        lines.append("")

        if poor > 0:
            lines.append("⚠ WARNING: Some benchmarks have high variance (CV > 20%)")
            lines.append("  Possible causes:")
            lines.append("    - Thermal throttling (increase cooldown)")
            lines.append("    - CPU contention (reduce parallel jobs)")
            lines.append("    - Background processes (close applications)")
            lines.append("    - Insufficient rounds (increase to 5)")
            lines.append("")

        lines.append("=" * 100)

        return "\n".join(lines)

    def save_aggregated_results(self, benchmark_times: Dict[str, List[float]],
                                 output_dir: Path):
        """Save aggregated results in Criterion-compatible format."""
        output_dir.mkdir(parents=True, exist_ok=True)

        for benchmark_name, times in benchmark_times.items():
            median = statistics.median(times)
            mad = self.median_absolute_deviation(times)

            result = {
                "benchmark": benchmark_name,
                "median_ns": median,
                "mad_ns": mad,
                "cv_percent": self.coefficient_of_variation(times),
                "rounds": len(times),
                "all_times_ns": times,
            }

            output_file = output_dir / f"{benchmark_name}.json"
            with open(output_file, "w") as f:
                json.dump(result, f, indent=2)

        print(f"Saved aggregated results to: {output_dir}")

    def run(self):
        """Main aggregation workflow."""
        print(f"Collecting results from: {self.results_dir}")
        print(f"Expected rounds: {self.rounds}")
        print(f"Baseline prefix: {self.baseline_prefix}")
        print()

        # Collect results
        benchmark_times = self.collect_results()

        if not benchmark_times:
            print("Error: No benchmark results found.", file=sys.stderr)
            return 1

        # Generate report
        report = self.generate_report(benchmark_times)
        print(report)

        # Save aggregated results
        output_dir = self.results_dir / "aggregated"
        self.save_aggregated_results(benchmark_times, output_dir)

        return 0


def main():
    parser = argparse.ArgumentParser(
        description="Aggregate benchmark results across multiple rounds",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Aggregate 3 rounds with default settings
  python3 aggregate_benchmarks.py

  # Aggregate 5 rounds with custom baseline prefix
  python3 aggregate_benchmarks.py --rounds 5 --baseline-prefix "test"

  # Use custom results directory
  python3 aggregate_benchmarks.py --results-dir /path/to/criterion
        """
    )

    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path("target/criterion"),
        help="Criterion results directory (default: target/criterion)"
    )

    parser.add_argument(
        "--rounds",
        type=int,
        default=3,
        help="Number of benchmark rounds (default: 3)"
    )

    parser.add_argument(
        "--baseline-prefix",
        type=str,
        default="round",
        help="Baseline name prefix (default: round)"
    )

    args = parser.parse_args()

    aggregator = BenchmarkAggregator(
        results_dir=args.results_dir,
        rounds=args.rounds,
        baseline_prefix=args.baseline_prefix
    )

    return aggregator.run()


if __name__ == "__main__":
    sys.exit(main())
