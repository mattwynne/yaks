import runpy
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE = runpy.run_path(str(Path(__file__).resolve().parents[1] / "bin/compare-dependency-help"))


class ComparisonTests(unittest.TestCase):
    def test_unrecognized_task_names_are_reported_as_review_not_execution_errors(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "bin").mkdir()
            (root / "bin/dev").write_text("""eval-dependency-help() {
  mkdir -p "$YX_EVAL_ARTIFACTS"
  printf '[]' > "$YX_EVAL_ARTIFACTS/map.json"
  printf '{"outcome":"review","reasons":["Unclear task roles"]}' > "$YX_EVAL_ARTIFACTS/assessment.json"
  return 2
}
""")
            row = MODULE["run_attempt"](root, root / "yx", "new", 1, root, "test", 5)
            self.assertEqual(row["outcome"], "review")
            self.assertEqual(MODULE["summarize"]([row])["new"]["review"], 1)

    def test_current_only_evaluates_the_working_copy_without_building_a_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            binary = path / "yx"
            binary.write_text("test binary")
            output = path / "results"
            with patch.dict(MODULE["main"].__globals__, {
                "build_current_binary": lambda *args: (binary, "current-sha"),
                "build_binary": lambda *args: self.fail("Baseline must not be built"),
                "snapshot_help": lambda *args: "current help",
                "run_attempt": lambda *args: {
                    "variant": "new", "attempt": 1, "outcome": "pass",
                    "seconds": 1, "artifacts": "test",
                },
            }):
                with patch("sys.argv", ["eval", "--current-only", "--working-tree",
                                       "--runs", "1", "--output", str(output)]):
                    self.assertEqual(MODULE["main"](), 0)
            report = MODULE["json"].loads((output / "results.json").read_text())
            self.assertNotIn("old", report["summary"])
            self.assertEqual(report["summary"]["new"]["passed"], 1)
            self.assertTrue(report["working_tree"])

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
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "bin").mkdir()
            script = root / "bin/dev"
            script.write_text("eval-dependency-help() { return 1; }\n")
            row = MODULE["run_attempt"](root, root / "yx", "new", 1, root, "test", 5)
            self.assertEqual(row["outcome"], "error")
            script.write_text("""eval-dependency-help() {
  mkdir -p "$YX_EVAL_ARTIFACTS"
  printf '[]' > "$YX_EVAL_ARTIFACTS/map.json"
  printf '{"outcome":"fail","reasons":["Flat map"]}' > "$YX_EVAL_ARTIFACTS/assessment.json"
  return 1
}
""")
            row = MODULE["run_attempt"](root, root / "yx", "new", 2, root, "test", 5)
            self.assertEqual(row["outcome"], "fail")

    def test_summary_counts_all_attempts(self):
        summarize = MODULE["summarize"]
        rows = [
            {"variant": "old", "outcome": "pass", "seconds": 20},
            {"variant": "old", "outcome": "fail", "seconds": 40},
            {"variant": "old", "outcome": "error", "seconds": 60},
            {"variant": "new", "outcome": "pass", "seconds": 10},
        ]
        self.assertEqual(summarize(rows)["old"], {
            "attempts": 3, "passed": 1, "failed": 1, "errors": 1, "review": 0,
            "pass_rate": 1 / 3, "median_seconds": 40,
        })


if __name__ == "__main__":
    unittest.main()
