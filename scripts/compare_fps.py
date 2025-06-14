import json
import sys
import os

if len(sys.argv) != 5:
    print("Usage: compare_fps.py NEW BASELINE OUTPUT THRESHOLD")
    sys.exit(1)

new_file, baseline_file, output_file, threshold = sys.argv[1:]
threshold = float(threshold)

if not os.path.exists(new_file):
    print("benchmark_result.json not found, run parse_perf_log.py first")
    sys.exit(1)

with open(new_file) as f:
    new_data = json.load(f)
new_fps = float(new_data.get("fps", 0))

if new_fps == 0:
    print("Warning: FPS is zero, logs might be incorrect")

if os.path.exists(baseline_file):
    with open(baseline_file) as f:
        base_data = json.load(f)
    base_fps = float(base_data.get("fps", 0))
else:
    base_fps = new_fps
    with open(baseline_file, "w") as f:
        json.dump(new_data, f)

with open(output_file, "w") as f:
    json.dump(new_data, f)

if base_fps and new_fps < base_fps * (1 - threshold / 100.0):
    print(f"FPS decreased from {base_fps} to {new_fps}")
    sys.exit(1)
else:
    print(f"FPS {new_fps}, baseline {base_fps}")
