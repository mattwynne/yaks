import runpy
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE = runpy.run_path(str(Path(__file__).resolve().parents[1] / "bin/compare-dependency-help"))


class ComparisonTests(unittest.TestCase):
    def test_archive_timestamps_are_refreshed_before_using_shared_build_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            target = output / "target"
            source_file = output / "source-old/src/main.rs"

            def fake_run(command, **kwargs):
                if command[0] == "tar":
                    source_file.parent.mkdir(parents=True)
                    source_file.write_text("old source")
                    os.utime(source_file, (1, 1))
                elif command[0] == "cargo":
                    self.assertGreater(source_file.stat().st_mtime, 1)
                    (target / "debug").mkdir(parents=True)
                    (target / "debug/yx").write_text("fake binary")

            with patch.object(MODULE["subprocess"], "check_output", return_value="abc\n"):
                with patch.object(MODULE["subprocess"], "run", side_effect=fake_run):
                    binary, revision = MODULE["build_binary"](
                        output, "HEAD", "old", output, target)
            self.assertEqual(revision, "abc")
            self.assertEqual(binary.read_text(), "fake binary")

    def test_grader_failures_and_execution_errors_are_separate(self):
        classify = MODULE["classify"]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "map.json"
            self.assertEqual(classify(path, 1), "error")
            path.write_text("[]")
            self.assertEqual(classify(path, 1), "fail")
            self.assertEqual(classify(path, 0), "pass")
            path.write_text("")
            self.assertEqual(classify(path, 1), "error")

    def test_summary_counts_all_attempts(self):
        summarize = MODULE["summarize"]
        rows = [
            {"variant": "old", "outcome": "pass", "seconds": 20},
            {"variant": "old", "outcome": "fail", "seconds": 40},
            {"variant": "old", "outcome": "error", "seconds": 60},
            {"variant": "new", "outcome": "pass", "seconds": 10},
        ]
        self.assertEqual(summarize(rows)["old"], {
            "attempts": 3, "passed": 1, "failed": 1, "errors": 1,
            "pass_rate": 1 / 3, "median_seconds": 40,
        })


if __name__ == "__main__":
    unittest.main()
