#!/usr/bin/env python3
"""
Fast-TA Benchmark Runner

A modern CLI tool for running and comparing fast-ta vs ta-lib benchmarks.
Uses Criterion for benchmarking and provides clear, colorful comparison tables.
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional
import statistics

import typer
from rich.console import Console
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn
from rich.panel import Panel
from rich import box
from rich.text import Text

app = typer.Typer(
    name="benchmark",
    help="Run and compare fast-ta vs ta-lib benchmarks",
    add_completion=False,
)

console = Console()

# Default sizes for benchmarking
DEFAULT_SIZES = [100, 1000, 10000, 100000]
DEFAULT_ROUNDS = 3

# All available indicators
ALL_INDICATORS = [
    "sma", "ema", "dema", "tema", "trima", "wma", "trix",
    "rsi", "roc", "mom", "apo", "macd",
    "obv", "ad", "bop", "atr", "trange", "bollinger",
    "midpoint", "midprice", "tsf", "linearreg", "t3",
    "adx", "dx", "aroon", "cci", "cmo", "kama",
    "mfi", "stochastic", "stochastic_fast", "williams_r", "ultosc",
    "var",
]


class BenchmarkRunner:
    """Handles running benchmarks and parsing results."""

    def __init__(self, results_dir: Path):
        self.results_dir = results_dir
        self.project_root = Path(__file__).parent.parent

    def run_benchmark(
        self,
        indicator: str,
        baseline: str,
        show_output: bool = False,
        size: int = 100000,
    ) -> bool:
        """Run a single benchmark with Criterion."""
        # Filter to specific size to reduce benchmark time
        # Pattern: "indicator/impl/size" or "indicator/impl_p{period}/size"
        filter_pattern = f"{indicator}/.*/{size}"

        cmd = [
            "cargo",
            "bench",
            "--bench",
            "talib_comparison",
            filter_pattern,
            "--",
            "--save-baseline",
            baseline,
        ]

        try:
            result = subprocess.run(
                cmd,
                cwd=self.project_root,
                capture_output=not show_output,
                text=True,
                timeout=900,  # 15 minute timeout for multi-period benchmarks (4 sizes × 5 periods × 2 impls = 40 runs)
            )
            return result.returncode == 0
        except subprocess.TimeoutExpired:
            console.print(f"[red]Timeout running {indicator}[/red]")
            return False

    def get_period_times(
        self, indicator: str, impl: str, size: int, baseline: str
    ) -> Dict[str, float]:
        """Extract median times for all periods from Criterion JSON estimates.

        Returns a dict mapping period names (e.g., 'p5', 'p21', 'overall') to times in µs.
        For single-period benchmarks, returns {'overall': time}.
        For multi-period benchmarks, returns {'p5': time, 'p21': time, ..., 'overall': median}.
        """
        import re

        # Structure 1: Multi-period indicator/impl_p{period}/size/baseline/estimates.json
        # Check this FIRST to prefer multi-period results when both exist
        period_times = {}
        indicator_dir = self.results_dir / indicator

        if indicator_dir.exists():
            # Find all impl_p{period} directories
            pattern = re.compile(rf"^{re.escape(impl)}_p(\d+)$")

            for dir_path in indicator_dir.iterdir():
                if dir_path.is_dir():
                    match = pattern.match(dir_path.name)
                    if match:
                        period = match.group(1)
                        estimates_path = dir_path / str(size) / baseline / "estimates.json"
                        if estimates_path.exists():
                            try:
                                with open(estimates_path) as f:
                                    data = json.load(f)
                                    time_us = data["median"]["point_estimate"] / 1000.0
                                    period_times[f"p{period}"] = time_us
                            except (json.JSONDecodeError, KeyError, FileNotFoundError):
                                continue

            if period_times:
                # Add overall median
                period_times["overall"] = statistics.median(period_times.values())
                return period_times

        # Structure 2: Single-period indicator/impl/size/baseline/estimates.json
        # Only use this if no multi-period results exist
        single_period_path = self.results_dir / indicator / impl / str(size) / baseline / "estimates.json"

        if single_period_path.exists():
            try:
                with open(single_period_path) as f:
                    data = json.load(f)
                    time_us = data["median"]["point_estimate"] / 1000.0
                    return {"overall": time_us}
            except (json.JSONDecodeError, KeyError, FileNotFoundError):
                pass

        return {}

    def detect_latest_baseline(self, indicator: str = "stochastic") -> Optional[str]:
        """Auto-detect the most recent baseline for an indicator."""
        indicator_dir = self.results_dir / indicator / "fast-ta" / "100000"

        if not indicator_dir.exists():
            return None

        # Find all baseline directories (exclude 'report', 'new', 'change', 'base')
        excluded = {'report', 'new', 'change', 'base'}
        baselines = [
            d.name for d in indicator_dir.iterdir()
            if d.is_dir() and d.name not in excluded
        ]

        if not baselines:
            return None

        # Prefer 'comparison' baseline if it exists (default for run command)
        if 'comparison' in baselines:
            return 'comparison'

        # Then prefer round3_* baselines if they exist
        round3 = [b for b in baselines if b.startswith('round3_')]
        if round3:
            return round3[0]

        # Otherwise use the most recently modified
        baseline_paths = [(b, (indicator_dir / b).stat().st_mtime) for b in baselines]
        baseline_paths.sort(key=lambda x: x[1], reverse=True)
        return baseline_paths[0][0] if baseline_paths else None

    def collect_results(
        self, indicators: List[str], baseline: str, size: int = 100000
    ) -> Dict[str, Dict[str, Dict[str, float]]]:
        """Collect benchmark results for specified indicators.

        Returns structure:
        {
            "indicator_name": {
                "fast-ta": {"p5": 123.4, "p21": 125.6, ..., "overall": 124.5},
                "ta-lib": {"p5": 200.1, "p21": 202.3, ..., "overall": 201.2}
            }
        }
        """
        # Auto-detect baseline if requested
        if baseline == "auto":
            detected = self.detect_latest_baseline(indicators[0] if indicators else "stochastic")
            if detected:
                baseline = detected
                console.print(f"[dim]Using baseline: {baseline}[/dim]")
            else:
                console.print("[yellow]Could not auto-detect baseline, using 'comparison'[/yellow]")
                baseline = "comparison"

        results = {}

        for indicator in indicators:
            fast_ta = self.get_period_times(indicator, "fast-ta", size, baseline)
            ta_lib = self.get_period_times(indicator, "ta-lib", size, baseline)

            if fast_ta and ta_lib:
                results[indicator] = {
                    "fast-ta": fast_ta,
                    "ta-lib": ta_lib,
                }

        return results


def format_time(us: float) -> str:
    """Format microseconds in human-readable form."""
    if us < 1.0:
        return f"{us * 1000:.2f} ns"
    elif us < 1000:
        return f"{us:.2f} µs"
    elif us < 1000000:
        return f"{us / 1000:.2f} ms"
    else:
        return f"{us / 1000000:.2f} s"


def create_comparison_table(
    results: Dict[str, Dict[str, Dict[str, float]]],
    sort_by: str = "ratio"
) -> Table:
    """Create a rich table comparing fast-ta vs ta-lib with per-period analysis."""

    # Determine which periods are present
    all_periods = set()
    for indicator_data in results.values():
        for impl_data in indicator_data.values():
            all_periods.update(k for k in impl_data.keys() if k != "overall")

    # Sort periods numerically
    periods = sorted(all_periods, key=lambda p: int(p[1:]) if p[1:].isdigit() else 0)

    table = Table(
        title="Fast-TA vs TA-Lib Benchmark Comparison (100k elements)",
        box=box.ROUNDED,
        show_header=True,
        header_style="bold cyan",
    )

    table.add_column("Indicator", style="bright_white")

    # Add columns for each period
    for period in periods:
        table.add_column(period.upper(), justify="right")

    table.add_column("Overall", justify="right")
    table.add_column("Analysis")

    # Compute statistics
    comparisons = []
    for indicator, data in results.items():
        fast_ta = data["fast-ta"]
        ta_lib = data["ta-lib"]

        # Calculate ratios for each period
        period_ratios = {}
        for period in periods:
            if period in fast_ta and period in ta_lib:
                period_ratios[period] = fast_ta[period] / ta_lib[period]

        # Overall ratio
        overall_ratio = fast_ta.get("overall", 0) / ta_lib.get("overall", 1)

        # Analyze period-dependency
        if period_ratios:
            ratios_values = [v for v in period_ratios.values()]
            min_ratio = min(ratios_values)
            max_ratio = max(ratios_values)
            ratio_range = max_ratio - min_ratio
            ratio_variance = statistics.stdev(ratios_values) if len(ratios_values) > 1 else 0
        else:
            min_ratio = max_ratio = overall_ratio
            ratio_range = 0
            ratio_variance = 0

        comparisons.append({
            "indicator": indicator,
            "period_ratios": period_ratios,
            "overall_ratio": overall_ratio,
            "min_ratio": min_ratio,
            "max_ratio": max_ratio,
            "ratio_range": ratio_range,
            "ratio_variance": ratio_variance,
        })

    # Sort
    if sort_by == "ratio":
        comparisons.sort(key=lambda x: x["overall_ratio"], reverse=True)
    else:  # alphabetical
        comparisons.sort(key=lambda x: x["indicator"])

    # Add rows
    for comp in comparisons:
        row_data = [comp["indicator"]]

        # Add period ratio cells
        for period in periods:
            if period in comp["period_ratios"]:
                ratio = comp["period_ratios"][period]
                ratio_text = Text(f"{ratio:.2f}x")
                if ratio < 0.98:
                    ratio_text.stylize("bold green")
                elif ratio > 1.02:
                    ratio_text.stylize("bold red")
                else:
                    ratio_text.stylize("yellow")
                row_data.append(ratio_text)
            else:
                row_data.append("")

        # Overall ratio
        overall_ratio = comp["overall_ratio"]
        overall_text = Text(f"{overall_ratio:.2f}x")
        if overall_ratio < 0.98:
            overall_text.stylize("bold green")
        elif overall_ratio > 1.02:
            overall_text.stylize("bold red")
        else:
            overall_text.stylize("yellow")
        row_data.append(overall_text)

        # Analysis with insights
        analysis_parts = []

        # Performance status
        if overall_ratio < 0.98:
            speedup = (1.0 / overall_ratio - 1.0) * 100
            analysis_parts.append(Text(f"✓ {speedup:.0f}% faster", style="green"))
        elif overall_ratio > 1.02:
            slowdown = (overall_ratio - 1.0) * 100
            analysis_parts.append(Text(f"✗ {slowdown:.0f}% slower", style="red"))
        else:
            analysis_parts.append(Text("≈ competitive", style="yellow"))

        # Period dependency analysis
        if comp["ratio_range"] > 0.3:
            # Significant period-dependent behavior
            analysis_parts.append(Text(" | ⚠ Period-dependent", style="yellow"))
            if comp["min_ratio"] < 0.9 and comp["max_ratio"] > 1.1:
                analysis_parts.append(Text(" (hybrid?)", style="dim"))

        # Combine analysis parts
        analysis_text = Text()
        for part in analysis_parts:
            analysis_text.append_text(part)

        row_data.append(analysis_text)

        table.add_row(*row_data)

    return table


def print_summary(results: Dict[str, Dict[str, Dict[str, float]]]):
    """Print summary statistics."""
    total = len(results)
    if total == 0:
        console.print("[yellow]No results to summarize[/yellow]")
        return

    comparisons = []
    for data in results.values():
        fast_ta = data["fast-ta"].get("overall", 0)
        ta_lib = data["ta-lib"].get("overall", 1)
        ratio = fast_ta / ta_lib
        comparisons.append(ratio)

    faster = sum(1 for r in comparisons if r < 0.98)
    slower = sum(1 for r in comparisons if r > 1.02)
    equal = sum(1 for r in comparisons if 0.98 <= r <= 1.02)

    summary = Table(box=box.SIMPLE, show_header=False, padding=(0, 2))
    summary.add_column(style="cyan")
    summary.add_column(justify="right", style="bold")

    summary.add_row("Total Indicators", str(total))
    summary.add_row(
        "Faster (>2%)",
        f"[green]{faster}[/green] ({faster/total*100:.1f}%)"
    )
    summary.add_row(
        "Competitive (±2%)",
        f"[yellow]{equal}[/yellow] ({equal/total*100:.1f}%)"
    )
    summary.add_row(
        "Slower (>2%)",
        f"[red]{slower}[/red] ({slower/total*100:.1f}%)"
    )

    console.print(Panel(summary, title="Summary", border_style="cyan"))


@app.command()
def run(
    indicators: Optional[List[str]] = typer.Argument(
        None, help="Specific indicators to benchmark (default: all)"
    ),
    size: int = typer.Option(100000, "--size", "-s", help="Data size to test"),
    sort_by: str = typer.Option(
        "ratio", "--sort", help="Sort results by: ratio, gap, name"
    ),
    show_output: bool = typer.Option(
        False, "--verbose", "-v", help="Show benchmark output"
    ),
    skip_build: bool = typer.Option(
        False, "--skip-build", help="Skip cargo build step"
    ),
    baseline: str = typer.Option(
        "comparison", "--baseline", "-b", help="Baseline name to save results"
    ),
):
    """
    Run benchmarks and compare fast-ta vs ta-lib performance.

    Examples:
        # Run all benchmarks
        ./scripts/benchmark.py run

        # Run specific indicators
        ./scripts/benchmark.py run sma ema rsi

        # Test different size
        ./scripts/benchmark.py run --size 10000

        # Show verbose output
        ./scripts/benchmark.py run stochastic -v
    """
    project_root = Path(__file__).parent.parent
    results_dir = project_root / "target" / "criterion"

    # Determine which indicators to run
    if indicators:
        # Validate indicator names
        invalid = [i for i in indicators if i not in ALL_INDICATORS]
        if invalid:
            console.print(f"[red]Unknown indicators: {', '.join(invalid)}[/red]")
            console.print(f"\nAvailable indicators:")
            console.print(f"  {', '.join(ALL_INDICATORS)}")
            raise typer.Exit(1)
        to_run = indicators
    else:
        to_run = ALL_INDICATORS

    console.print(
        Panel(
            f"[cyan]Running benchmarks for {len(to_run)} indicators[/cyan]\n"
            f"Size: {size:,}, Baseline: {baseline}",
            title="Benchmark Configuration",
        )
    )

    # Build if needed
    if not skip_build:
        console.print("\n[cyan]Building benchmarks...[/cyan]")
        result = subprocess.run(
            ["cargo", "bench", "--bench", "talib_comparison", "--no-run"],
            cwd=project_root,
            capture_output=not show_output,
        )
        if result.returncode != 0:
            console.print("[red]Build failed[/red]")
            raise typer.Exit(1)
        console.print("[green]✓ Build complete[/green]")

    # Run benchmarks
    runner = BenchmarkRunner(results_dir)

    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TaskProgressColumn(),
        console=console,
    ) as progress:

        task = progress.add_task("Running benchmarks", total=len(to_run))

        for indicator in to_run:
            progress.update(task, description=f"Benchmarking {indicator}")

            success = runner.run_benchmark(indicator, baseline, show_output, size)
            if not success and not show_output:
                console.print(f"[yellow]Warning: {indicator} failed[/yellow]")

            progress.advance(task)

    # Collect and display results
    console.print("\n[cyan]Collecting results...[/cyan]")
    results = runner.collect_results(to_run, baseline, size=size)

    if not results:
        console.print("[red]No results found. Benchmarks may have failed.[/red]")
        console.print("[yellow]Tip: Try running with --verbose to see errors[/yellow]")
        raise typer.Exit(1)

    console.print()
    print_summary(results)
    console.print()

    table = create_comparison_table(results, sort_by=sort_by)
    console.print(table)


@app.command()
def results(
    indicators: Optional[List[str]] = typer.Argument(
        None, help="Specific indicators to show (default: all)"
    ),
    size: int = typer.Option(100000, "--size", "-s", help="Data size to display"),
    sort_by: str = typer.Option(
        "ratio", "--sort", help="Sort results by: ratio, gap, name"
    ),
    baseline: str = typer.Option(
        "auto", "--baseline", "-b", help="Baseline name to display (auto=detect latest)"
    ),
):
    """
    Display results from previous benchmark runs without re-running.

    Examples:
        # Show all results
        ./scripts/benchmark.py results

        # Show specific indicators
        ./scripts/benchmark.py results sma ema

        # Sort by absolute gap
        ./scripts/benchmark.py results --sort gap
    """
    project_root = Path(__file__).parent.parent
    results_dir = project_root / "target" / "criterion"

    if not results_dir.exists():
        console.print("[red]No benchmark results found. Run benchmarks first:[/red]")
        console.print("  ./scripts/benchmark.py run")
        raise typer.Exit(1)

    # Determine which indicators to show
    to_show = indicators if indicators else ALL_INDICATORS

    runner = BenchmarkRunner(results_dir)
    results = runner.collect_results(to_show, baseline, size=size)

    if not results:
        console.print(f"[yellow]No results found for baseline '{baseline}'[/yellow]")
        console.print("\nTip: Check available baselines:")
        console.print(f"  ls {results_dir}/*/fast-ta/100000/")
        raise typer.Exit(1)

    print_summary(results)
    console.print()

    table = create_comparison_table(results, sort_by=sort_by)
    console.print(table)


@app.command()
def list():
    """List all available indicators."""
    console.print("\n[cyan]Available Indicators:[/cyan]")
    for i, ind in enumerate(sorted(ALL_INDICATORS), 1):
        console.print(f"  {i:2}. {ind}")
    console.print(f"\n[dim]Total: {len(ALL_INDICATORS)} indicators[/dim]\n")


if __name__ == "__main__":
    app()
