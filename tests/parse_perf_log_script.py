import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from parse_perf_log import parse
from perf_schema import COUNTS, SAMPLES, FRAMES


def measurements():
    return [{"candles": count, "sample": sample, "frames": FRAMES, "fps": 60 + sample}
            for count in COUNTS for sample in range(SAMPLES)]


def log(rows):
    return "\n".join("PERF_SAMPLE " + json.dumps(row) for row in rows)


class ScriptTests(unittest.TestCase):
    def test_complete_measurements(self):
        data = parse("unrelated log\n" + log(measurements()))
        self.assertEqual(data["cases"]["1000"], [60, 61, 62, 63, 64])

    def test_missing_samples_or_workloads(self):
        for rows in ([], measurements()[:-1], measurements()[5:]):
            with self.subTest(rows=len(rows)), self.assertRaises(ValueError):
                parse(log(rows))

    def test_invalid_samples(self):
        for value in (0, -1, float("nan"), float("inf"), None, True, "60"):
            rows = measurements()
            rows[0]["fps"] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                parse(log(rows))

    def test_wrong_count_index_frames_and_duplicates(self):
        for field, value in (("candles", 100), ("sample", 5), ("sample", True), ("frames", 0)):
            rows = measurements()
            rows[0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                parse(log(rows))
        with self.assertRaises(ValueError):
            parse(log(measurements() + measurements()[:1]))

    def test_malformed_sample(self):
        with self.assertRaises(ValueError):
            parse(log(measurements()) + '\nPERF_SAMPLE {broken}')
