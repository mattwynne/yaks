# shellcheck shell=bash

Describe 'dependency-mapping help eval grader'
  It 'accepts a tree-first map with only shared and external blockers'
    cat > answer.json <<'EOF'
{
  "yaks": [
    {"name": "Launch catalog service", "parent": null},
    {"name": "Publish shared contract", "parent": "Launch catalog service"},
    {"name": "Deploy catalog service", "parent": "Launch catalog service"},
    {"name": "Wire components", "parent": "Deploy catalog service"},
    {"name": "Adapt legacy importer", "parent": "Wire components"},
    {"name": "Validate common library", "parent": "Adapt legacy importer"},
    {"name": "Implement reader", "parent": "Validate common library"},
    {"name": "Implement refresh", "parent": "Validate common library"},
    {"name": "Build search endpoint", "parent": "Wire components"}
  ],
  "blockers": [
    {"blocked": "Implement reader", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Implement refresh", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Build search endpoint", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Implement reader", "kind": "manual", "by": null, "reason": "Waiting for vendor credentials"},
    {"blocked": "Implement refresh", "kind": "manual", "by": null, "reason": "Waiting for vendor credentials"}
  ]
}
EOF

    When run sh -c "'$TEST_PROJECT_DIR/bin/grade-dependency-help-eval' < answer.json"
    The status should be success
    The output should include 'PASS: prerequisite nesting guidance'
  End

  It 'rejects a flat map joined by explicit blockers'
    cat > flat-answer.json <<'EOF'
{
  "yaks": [
    {"name": "Launch catalog service", "parent": null},
    {"name": "Publish shared contract", "parent": "Launch catalog service"},
    {"name": "Deploy catalog service", "parent": "Launch catalog service"},
    {"name": "Wire components", "parent": "Launch catalog service"},
    {"name": "Adapt legacy importer", "parent": "Launch catalog service"},
    {"name": "Validate common library", "parent": "Launch catalog service"},
    {"name": "Implement reader", "parent": "Launch catalog service"},
    {"name": "Implement refresh", "parent": "Launch catalog service"},
    {"name": "Build search endpoint", "parent": "Launch catalog service"}
  ],
  "blockers": [
    {"blocked": "Deploy catalog service", "kind": "yak", "by": "Wire components", "reason": null},
    {"blocked": "Wire components", "kind": "yak", "by": "Adapt legacy importer", "reason": null},
    {"blocked": "Wire components", "kind": "yak", "by": "Build search endpoint", "reason": null},
    {"blocked": "Adapt legacy importer", "kind": "yak", "by": "Validate common library", "reason": null}
  ]
}
EOF

    When run sh -c "'$TEST_PROJECT_DIR/bin/grade-dependency-help-eval' < flat-answer.json"
    The status should be failure
    The error should include 'FAIL: prerequisites are flattened into sibling yaks'
  End

  It 'rejects a parent-only blocker when the impediment affects child leaves'
    cat > parent-blocker-answer.json <<'EOF'
{
  "yaks": [
    {"name": "Launch catalog service", "parent": null},
    {"name": "Publish shared contract", "parent": "Launch catalog service"},
    {"name": "Deploy catalog service", "parent": "Launch catalog service"},
    {"name": "Wire components", "parent": "Deploy catalog service"},
    {"name": "Adapt legacy importer", "parent": "Wire components"},
    {"name": "Validate common library", "parent": "Adapt legacy importer"},
    {"name": "Implement reader", "parent": "Validate common library"},
    {"name": "Implement refresh", "parent": "Validate common library"},
    {"name": "Build search endpoint", "parent": "Wire components"}
  ],
  "blockers": [
    {"blocked": "Implement reader", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Implement refresh", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Build search endpoint", "kind": "yak", "by": "Publish shared contract", "reason": null},
    {"blocked": "Validate common library", "kind": "manual", "by": null, "reason": "Waiting for vendor credentials"}
  ]
}
EOF

    When run sh -c "'$TEST_PROJECT_DIR/bin/grade-dependency-help-eval' < parent-blocker-answer.json"
    The status should be failure
    The error should include 'FAIL: a parent blocker does not gate the affected leaves'
  End
End
