"""Compare each workload median against a separately reviewed measured baseline."""
import json
import math
import sys
from pathlib import Path
from statistics import median

from perf_schema import validate


def compare(current, baseline, threshold):
    validate(current)
    validate(baseline)
    if not math.isfinite(threshold) or not 0 <= threshold < 100:
        raise ValueError("threshold must be finite and in [0, 100)")
    results = {}
    for count, samples in current["cases"].items():
        actual, expected = median(samples), median(baseline["cases"][count])
        results[count] = {
            "median_fps": actual,
            "baseline_median_fps": expected,
            "change_percent": (actual / expected - 1) * 100,
            "passed": actual >= expected * (1 - threshold / 100),
        }
    return {"threshold_percent": threshold, "cases": results,
            "passed": all(case["passed"] for case in results.values())}


def main():
    if len(sys.argv) != 5:
        raise ValueError("Usage: compare_fps.py NEW BASELINE OUTPUT THRESHOLD")
    current, baseline, output, threshold = sys.argv[1:]
    result = compare(json.loads(Path(current).read_text(encoding="utf-8")),
                     json.loads(Path(baseline).read_text(encoding="utf-8")), float(threshold))
    Path(output).write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, TypeError, KeyError, OSError) as error:
        sys.exit(f"Invalid performance comparison: {error}")
