"""Pytest configuration for the liq-ta Python bindings package."""

from pathlib import Path
import sys


PACKAGE_ROOT = Path(__file__).resolve().parents[1] / "python"

if str(PACKAGE_ROOT) not in map(str, sys.path):
    sys.path.insert(0, str(PACKAGE_ROOT))
