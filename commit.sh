#!/bin/bash
git add -A
git commit -m "Tighten cyclomatic complexity threshold to 34

- Lowered max_cyclomatic from 40 to 34
- Refactored route_command to reduce complexity:
  - Extracted handle_add_command helper for Add branch logic
  - Extracted handle_tag_command helper for Tag action routing
  - Reduced nested matches and complex conditionals
- route_command no longer in top 5 most complex functions"
