import subprocess
import sys
from pathlib import Path


def test_missing_file(tmp_path):
    missing = tmp_path / "benchmark_result.json"
    baseline = tmp_path / "baseline.json"
    output = tmp_path / "result.json"
    result = subprocess.run(
        [sys.executable, str(Path(__file__).resolve().parent.parent / "scripts" / "compare_fps.py"),
         str(missing), str(baseline), str(output), "5"],
        capture_output=True, text=True
    )
    assert result.returncode == 1
    assert "benchmark_result.json not found" in result.stdout

