#!/usr/bin/env python3
"""Grade the catalog prose case by task roles, ancestry, and blockers."""

import argparse
import json
import re
import sys


def flatten(nodes):
    result = []
    for node in nodes:
        result.append(node)
        result.extend(flatten(node.get("children", [])))
    return result


def role(name):
    name = name.lower()
    if "contract" in name:
        return "contract"
    if "reader" in name:
        return "reader"
    if "refresh" in name:
        return "refresh"
    if "validat" in name and re.search(r"sdk|library", name):
        return "validation"
    if re.search(r"integrat|wir", name):
        return "integration"
    if "deploy" in name:
        return "deployment"
    if "importer" in name and re.search(r"adapt|migrat|convert|port", name):
        return "importer"
    if re.search(r"search|endpoint", name):
        return "endpoint"
    return None


def grade(data):
    if not isinstance(data, list):
        return {"outcome": "review", "reasons": ["Expected a JSON yak list"]}
    nodes = flatten(data)
    if not nodes:
        return {"outcome": "fail", "reasons": [
            "map did not use the expected prerequisite tree: no yaks were created"
        ]}
    roles = {}
    for node in nodes:
        task_role = role(node["name"])
        if task_role:
            roles.setdefault(task_role, []).append(node)
        elif not (node.get("parent_id") is None
                  and re.search(r"launch|release", node["name"], re.IGNORECASE)):
            return {"outcome": "review", "reasons": [
                "Unrecognized task: " + node["name"]
            ]}
    required = ("deployment", "integration", "importer", "endpoint",
                "validation", "reader", "refresh", "contract")
    unclear = [key for key in required if len(roles.get(key, [])) != 1]
    if unclear:
        return {"outcome": "review", "reasons": [
            "Cannot identify exactly one task for: " + ", ".join(unclear)
        ]}
    tasks = {key: roles[key][0] for key in required}
    by_id = {node["id"]: node for node in nodes}
    if len(by_id) != len(nodes):
        return {"outcome": "review", "reasons": ["Duplicate task IDs"]}

    def beneath(child, parent):
        current = child.get("parent_id")
        visited = set()
        while current is not None and current not in visited:
            if current == parent["id"]:
                return True
            visited.add(current)
            current = by_id.get(current, {}).get("parent_id")
        return False

    reasons = []
    for earlier, later in (
        ("integration", "deployment"), ("importer", "integration"),
        ("endpoint", "integration"), ("validation", "importer"),
        ("reader", "validation"), ("refresh", "validation"),
    ):
        if not beneath(tasks[earlier], tasks[later]):
            reasons.append(f"{earlier} must be nested beneath {later}")
    expected_yak_blockers = {
        (tasks[key]["id"], tasks["contract"]["id"])
        for key in ("reader", "refresh", "endpoint")
        if not beneath(tasks["contract"], tasks[key])
    }
    actual_yak_blockers = []
    manual_blockers = []
    for node in nodes:
        for blocker in node.get("blocked_by", []):
            if blocker["kind"] == "yak":
                actual_yak_blockers.append((node["id"], blocker["id"]))
            elif blocker["kind"] == "manual":
                manual_blockers.append((node["id"], blocker.get("reason")))
            else:
                reasons.append("Unrecognized blocker kind")
    if set(actual_yak_blockers) != expected_yak_blockers:
        reasons.append("Use explicit yak blockers only for the shared contract where nesting cannot express it")
    if len(set(actual_yak_blockers)) != len(actual_yak_blockers):
        reasons.append("Duplicate explicit yak blockers")
    expected_manual = {tasks["reader"]["id"], tasks["refresh"]["id"]}
    if ({task for task, _ in manual_blockers} != expected_manual
            or len(manual_blockers) != 2
            or not all(reason for _, reason in manual_blockers)):
        reasons.append("Both reader and refresh need their own manual credential blocker")
    if any(node.get("state") != "todo" for node in nodes):
        reasons.append("Planning work must remain todo")
    return {
        "outcome": "fail" if reasons else "pass",
        "reasons": reasons,
        "roles": {key: node["name"] for key, node in tasks.items()},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", help="Save machine-readable assessment")
    args = parser.parse_args()
    try:
        assessment = grade(json.load(sys.stdin))
    except (ValueError, TypeError, KeyError) as error:
        assessment = {"outcome": "review", "reasons": [str(error)]}
    if args.output:
        from pathlib import Path
        Path(args.output).write_text(json.dumps(assessment, indent=2) + "\n")
    if assessment["outcome"] == "pass":
        print("PASS: prerequisite nesting guidance")
        return 0
    prefix = "REVIEW" if assessment["outcome"] == "review" else "FAIL"
    print(prefix + ": " + "; ".join(assessment["reasons"]), file=sys.stderr)
    return 2 if prefix == "REVIEW" else 1


if __name__ == "__main__":
    raise SystemExit(main())
