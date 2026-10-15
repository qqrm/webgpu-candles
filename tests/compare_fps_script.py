import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from compare_fps import compare
from perf_schema import COUNTS, WORKLOAD


def data(fps=100):
    return {"schema": 1, "workload": WORKLOAD,
            "cases": {str(count): [fps] * 5 for count in COUNTS}}


class ScriptTests(unittest.TestCase):
    def test_each_workload_is_required(self):
        current = data(200)
        current["cases"]["50000"] = [60] * 5
        self.assertFalse(compare(current, data(), 30)["passed"])

    def test_median_rejects_outlier_influence(self):
        current = data()
        current["cases"]["1000"] = [1, 98, 99, 100, 10000]
        self.assertTrue(compare(current, data(), 5)["passed"])

    def test_threshold_boundary(self):
        self.assertTrue(compare(data(70), data(), 30)["passed"])
        self.assertFalse(compare(data(69.9), data(), 30)["passed"])

    def test_invalid_baselines_and_thresholds(self):
        invalid = [data(0), data(float("nan")), {"fps": 0}, {}]
        mismatch = copy.deepcopy(data())
        mismatch["workload"] = "different"
        invalid.append(mismatch)
        for baseline in invalid:
            with self.subTest(baseline=baseline), self.assertRaises(ValueError):
                compare(data(), baseline, 30)
        for threshold in (-1, 100, float("nan"), float("inf")):
            with self.subTest(threshold=threshold), self.assertRaises(ValueError):
                compare(data(), data(), threshold)

    def test_missing_baseline_does_not_self_initialize(self):
        with TemporaryDirectory() as directory:
            current = Path(directory) / "current.json"
            baseline = Path(directory) / "missing.json"
            output = Path(directory) / "result.json"
            current.write_text(json.dumps(data()), encoding="utf-8")
            result = subprocess.run([sys.executable, str(Path(__file__).resolve().parents[1]
                                    / "scripts" / "compare_fps.py"), str(current),
                                    str(baseline), str(output), "30"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(baseline.exists())
            self.assertFalse(output.exists())
