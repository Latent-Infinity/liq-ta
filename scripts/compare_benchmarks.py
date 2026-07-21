#!/usr/bin/env python3
"""
Compare aggregated benchmark results between two baseline runs.

Usage:
    python3 scripts/compare_benchmarks.py \\
        --baseline-old round \\
        --baseline-new after \\
        --rounds 3
"""

import argparse
import sys
from pathlib import Path
from typing import Dict


class BenchmarkComparator:
    """Compare benchmark results between two baseline runs."""

    def __init__(self, results_dir: Path, baseline_old: str, baseline_new: str, rounds: int):
        self.results_dir = Path(results_dir)
        self.baseline_old = baseline_old
        self.baseline_new = baseline_new
        self.rounds = rounds

    def collect_baseline_results(self, baseline_prefix: str) -> Dict[str, float]:
        """Collect median results for a baseline across all rounds."""
        from aggregate_benchmarks import BenchmarkAggregator
        import statistics

        aggregator = BenchmarkAggregator(
            results_dir=self.results_dir,
            rounds=self.rounds,
            baseline_prefix=baseline_prefix
        )

        benchmark_times = aggregator.collect_results()

        # Compute median across rounds for each benchmark
        medians = {}
        for name, times in benchmark_times.items():
            if times:
                medians[name] = statistics.median(times)

        return medians

    def compare(self) -> str:
        """Generate comparison report."""
        print(f"Collecting baseline: {self.baseline_old}...")
        old_results = self.collect_baseline_results(self.baseline_old)

        print(f"Collecting baseline: {self.baseline_new}...")
        new_results = self.collect_baseline_results(self.baseline_new)

        if not old_results:
            return f"Error: No results found for baseline: {self.baseline_old}"

        if not new_results:
            return f"Error: No results found for baseline: {self.baseline_new}"

        # Find common benchmarks
        common = set(old_results.keys()) & set(new_results.keys())
        only_old = set(old_results.keys()) - set(new_results.keys())
        only_new = set(new_results.keys()) - set(old_results.keys())

        if not common:
            return "Error: No common benchmarks found between baselines"

        lines = []
        lines.append("=" * 120)
        lines.append("BENCHMARK COMPARISON REPORT")
        lines.append("=" * 120)
        lines.append(f"Baseline (old): {self.baseline_old}")
        lines.append(f"Baseline (new): {self.baseline_new}")
        lines.append(f"Rounds: {self.rounds}")
        lines.append(f"Common benchmarks: {len(common)}")
        lines.append("")

        # Compute changes
        results = []
        for name in sorted(common):
            old_time = old_results[name]
            new_time = new_results[name]
            change_ns = new_time - old_time
            change_pct = ((new_time - old_time) / old_time) * 100

            results.append({
                "name": name,
                "old": old_time,
                "new": new_time,
                "change_ns": change_ns,
                "change_pct": change_pct,
            })

        # Sort by absolute percentage change (largest first)
        results.sort(key=lambda x: abs(x["change_pct"]), reverse=True)

        # Print comparison table
        lines.append(f"{'Benchmark':<30} {'Old':>12} {'New':>12} {'Change':>12} {'%Change':>10} {'Status':>12}")
        lines.append("-" * 120)

        improved = 0
        regressed = 0
        neutral = 0

        for r in results:
            old_str = self.format_time(r["old"])
            new_str = self.format_time(r["new"])
            change_str = self.format_time(abs(r["change_ns"]))

            # Determine status
            if r["change_pct"] < -2.0:  # >2% improvement
                status = "✓ Improved"
                improved += 1
            elif r["change_pct"] > 2.0:  # >2% regression
                status = "✗ Regressed"
                regressed += 1
            else:
                status = "≈ Neutral"
                neutral += 1

            # Format change with sign
            if r["change_pct"] < 0:
                change_sign = f"-{change_str}"
                pct_sign = f"{r['change_pct']:+.2f}%"
            else:
                change_sign = f"+{change_str}"
                pct_sign = f"{r['change_pct']:+.2f}%"

            lines.append(
                f"{r['name']:<30} "
                f"{old_str:>12} "
                f"{new_str:>12} "
                f"{change_sign:>12} "
                f"{pct_sign:>10} "
                f"{status:>12}"
            )

        lines.append("")
        lines.append("=" * 120)
        lines.append("SUMMARY")
        lines.append("-" * 120)
        lines.append(f"Total benchmarks: {len(results)}")
        lines.append(f"  Improved (>2% faster):   {improved:3} ({improved / len(results) * 100:.1f}%)")
        lines.append(f"  Neutral (±2%):           {neutral:3} ({neutral / len(results) * 100:.1f}%)")
        lines.append(f"  Regressed (>2% slower):  {regressed:3} ({regressed / len(results) * 100:.1f}%)")
        lines.append("")

        if only_old:
            lines.append(f"⚠ Benchmarks only in '{self.baseline_old}': {', '.join(sorted(only_old))}")
        if only_new:
            lines.append(f"⚠ Benchmarks only in '{self.baseline_new}': {', '.join(sorted(only_new))}")

        if regressed > 0:
            lines.append("")
            lines.append("⚠ WARNING: Some benchmarks have regressed (>2% slower)")
            lines.append("Review the changes and consider:")
            lines.append("  - Are the changes expected?")
            lines.append("  - Can the performance be recovered?")
            lines.append("  - Is the correctness improvement worth the performance cost?")

        lines.append("")
        lines.append("=" * 120)

        return "\n".join(lines)

    @staticmethod
    def format_time(ns: float) -> str:
        """Format nanoseconds in human-readable units."""
        if ns < 1_000:
            return f"{ns:.2f} ns"
        elif ns < 1_000_000:
            return f"{ns / 1_000:.2f} µs"
        elif ns < 1_000_000_000:
            return f"{ns / 1_000_000:.2f} ms"
        else:
            return f"{ns / 1_000_000_000:.2f} s"


def main():
    parser = argparse.ArgumentParser(
        description="Compare benchmark results between two baselines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Compare 'round' baseline (before) with 'after' baseline (after changes)
  python3 scripts/compare_benchmarks.py \\
      --baseline-old round \\
      --baseline-new after \\
      --rounds 3

  # Compare with custom results directory
  python3 scripts/compare_benchmarks.py \\
      --baseline-old baseline_v1 \\
      --baseline-new baseline_v2 \\
      --rounds 5 \\
      --results-dir /path/to/criterion
        """
    )

    parser.add_argument(
        "--baseline-old",
        type=str,
        required=True,
        help="Old baseline prefix (e.g., 'round')"
    )

    parser.add_argument(
        "--baseline-new",
        type=str,
        required=True,
        help="New baseline prefix (e.g., 'after')"
    )

    parser.add_argument(
        "--rounds",
        type=int,
        default=3,
        help="Number of rounds for each baseline (default: 3)"
    )

    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path("target/criterion"),
        help="Criterion results directory (default: target/criterion)"
    )

    args = parser.parse_args()

    comparator = BenchmarkComparator(
        results_dir=args.results_dir,
        baseline_old=args.baseline_old,
        baseline_new=args.baseline_new,
        rounds=args.rounds
    )

    report = comparator.compare()
    print(report)


if __name__ == "__main__":
    sys.exit(main())
