#!/bin/bash
git add -A
git commit -m "Tighten cognitive complexity threshold to 42

- Lowered max_cognitive from 60 to 42
- Refactored display_header_box to reduce complexity:
  - Extracted style_yak_item helper for consistent item styling
  - Extracted write_box_line and write_dim_line helpers
  - Reduced duplication in styling logic for headers and children
- Function no longer appears in top 5 most complex"
