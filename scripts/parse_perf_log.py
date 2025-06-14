import json
import re
import sys

if len(sys.argv) != 3:
    print("Usage: parse_perf_log.py LOG OUTPUT")
    sys.exit(1)

log_path, out_path = sys.argv[1:3]

fps_values = []
with open(log_path) as f:
    for line in f:
        m = re.search(r"\d+ candles: ([0-9.]+) FPS", line)
        if m:
            fps_values.append(float(m.group(1)))

if not fps_values:
    print("No FPS data found in log")
    sys.exit(1)

fps = sum(fps_values) / len(fps_values) if fps_values else 0.0

with open(out_path, "w") as f:
    json.dump({"fps": fps}, f)
