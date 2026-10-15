"""Parse every expected completed-frame sample; missing measurements are errors."""
import json
import sys
from pathlib import Path

from perf_schema import COUNTS, SAMPLES, FRAMES, WORKLOAD, validate


def parse(log):
    cases = {str(count): [None] * SAMPLES for count in COUNTS}
    for line in log.splitlines():
        if "PERF_SAMPLE " not in line:
            continue
        data = json.loads(line.split("PERF_SAMPLE ", 1)[1])
        count, sample = data["candles"], data["sample"]
        if type(count) is not int or count not in COUNTS:
            raise ValueError("unexpected candle count")
        if type(sample) is not int or sample not in range(SAMPLES):
            raise ValueError("unexpected sample index")
        if data["frames"] != FRAMES or type(data["frames"]) is not int:
            raise ValueError("unexpected frame count")
        if cases[str(count)][sample] is not None:
            raise ValueError("duplicate performance sample")
        cases[str(count)][sample] = data["fps"]
    result = {"schema": 1, "workload": WORKLOAD, "cases": cases}
    validate(result)
    return result


def main():
    if len(sys.argv) != 3:
        raise ValueError("Usage: parse_perf_log.py LOG OUTPUT")
    result = parse(Path(sys.argv[1]).read_text(encoding="utf-8"))
    Path(sys.argv[2]).write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError) as error:
        sys.exit(f"Invalid performance measurements: {error}")
