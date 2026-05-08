# shellcheck shell=bash
Describe 'bin/changelog-section'
  It 'extracts the requested version section'
    cat > CHANGELOG.md <<'EOF'
# Changelog

## [0.3.0] - 2026-05-08

### Added
- Newer change

## [0.2.0] - 2026-05-07

### Added
- First release note

### Fixed
- Bug fix

## [0.1.0] - 2026-05-06

- Older change
EOF

    When run changelog-section 0.2.0
    The status should be success
    The output should equal "$(cat <<'EOF'

### Added
- First release note

### Fixed
- Bug fix
EOF
)"
  End

  It 'accepts a version with a leading v'
    cat > CHANGELOG.md <<'EOF'
# Changelog

## [0.2.0] - 2026-05-07

- Release note
EOF

    When run changelog-section v0.2.0
    The status should be success
    The output should include "- Release note"
  End

  It 'fails clearly when the version section is missing'
    cat > CHANGELOG.md <<'EOF'
# Changelog

## [0.1.0] - 2026-05-06

- Older change
EOF

    When run changelog-section 0.2.0
    The status should be failure
    The error should include "Error: version 0.2.0 not found in CHANGELOG.md"
  End

  It 'fails clearly when the version section is only headings'
    cat > CHANGELOG.md <<'EOF'
# Changelog

## [0.2.0] - 2026-05-07

### Added

### Fixed

## [0.1.0] - 2026-05-06

- Older change
EOF

    When run changelog-section 0.2.0
    The status should be failure
    The error should include "Error: changelog section for version 0.2.0 is empty or only headings"
  End

  It 'can read a custom changelog path'
    cat > CUSTOMLOG.md <<'EOF'
# Changelog

## [0.2.0] - 2026-05-07

- Custom path note
EOF

    When run changelog-section 0.2.0 CUSTOMLOG.md
    The status should be success
    The output should include "- Custom path note"
  End
End
