import json
from pathlib import Path
import runpy
import unittest


ROOT = Path(__file__).resolve().parents[1]


class ProseGradingTests(unittest.TestCase):
    def test_recorded_agent_maps_match_the_agreed_structural_assessments(self):
        grade = runpy.run_path(str(ROOT / "evals/prerequisite-nesting/grade.py"))["grade"]
        for name, expected in (
            ("old-nested", "pass"), ("old-flat", "fail"),
            ("old-mixed", "fail"), ("old-mixed-sibling", "fail"),
            ("new-nested", "pass"),
        ):
            with self.subTest(name=name):
                data = json.loads((ROOT / f"tests/fixtures/eval-{name}.json").read_text())
                self.assertEqual(grade(data)["outcome"], expected)

    def test_ambiguous_names_require_review_instead_of_failing_the_map(self):
        grade = runpy.run_path(str(ROOT / "evals/prerequisite-nesting/grade.py"))["grade"]
        data = json.loads((ROOT / "tests/fixtures/eval-new-nested.json").read_text())
        for node in data:
            if "reader" in node["name"].lower():
                node["name"] = "Prepare the first SDK path"
        self.assertEqual(grade(data)["outcome"], "review")

    def test_a_parent_only_credential_blocker_does_not_pass(self):
        grade = runpy.run_path(str(ROOT / "evals/prerequisite-nesting/grade.py"))["grade"]
        data = json.loads((ROOT / "tests/fixtures/eval-new-nested.json").read_text())
        for node in data:
            node["blocked_by"] = [
                blocker for blocker in node["blocked_by"] if blocker["kind"] != "manual"
            ]
            if "Validate" in node["name"]:
                node["blocked_by"].append({"kind": "manual", "reason": "Waiting for credentials"})
        self.assertEqual(grade(data)["outcome"], "fail")

    def test_extra_work_the_grader_does_not_understand_requires_review(self):
        grade = runpy.run_path(str(ROOT / "evals/prerequisite-nesting/grade.py"))["grade"]
        data = json.loads((ROOT / "tests/fixtures/eval-new-nested.json").read_text())
        data.append({"id": "extra", "name": "Prepare the rollout",
                     "parent_id": data[0]["id"], "state": "todo", "blocked_by": []})
        self.assertEqual(grade(data)["outcome"], "review")


if __name__ == "__main__":
    unittest.main()
