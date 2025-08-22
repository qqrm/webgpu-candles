import json
import subprocess
import sys
from pathlib import Path


def test_no_fps_in_log(tmp_path):
    log = tmp_path / "log.txt"
    log.write_text("some output without fps\n")
    output = tmp_path / "result.json"
    result = subprocess.run(
        [sys.executable, str(Path(__file__).resolve().parent.parent / "scripts" / "parse_perf_log.py"),
         str(log), str(output)],
        capture_output=True, text=True
    )
    assert result.returncode == 0
    assert json.loads(output.read_text()) == {"fps": 0.0}
    assert "No FPS data found in log" in result.stdout
