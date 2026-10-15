"""Versioned workload contract shared by measurement parsing and comparison."""
import math

COUNTS = (1000, 50000, 1000000)
SAMPLES = 5
FRAMES = 30
WORKLOAD = "cached-chart-completed-frames-800x600-msaa4-v1"


def validate(data):
    if data.get("schema") != 1 or data.get("workload") != WORKLOAD:
        raise ValueError("incompatible performance schema or workload")
    cases = data.get("cases")
    if not isinstance(cases, dict) or set(cases) != {str(count) for count in COUNTS}:
        raise ValueError("missing or unexpected workloads")
    for count, samples in cases.items():
        if not isinstance(samples, list) or len(samples) != SAMPLES:
            raise ValueError(f"expected {SAMPLES} samples for {count} candles")
        for fps in samples:
            if type(fps) not in (int, float) or not math.isfinite(fps) or fps <= 0:
                raise ValueError(f"missing, non-finite or non-positive FPS for {count} candles")
    return data
