# shellcheck shell=bash

Describe 'check-macos-linkage'
  setup_linkage_check() {
    LINKAGE_TMP=$(mktemp -d)
    export LINKAGE_TMP
    mkdir -p "$LINKAGE_TMP/bin"
    : > "$LINKAGE_TMP/yx"
    chmod +x "$LINKAGE_TMP/yx"

    cat > "$LINKAGE_TMP/bin/otool" <<'EOF'
#!/usr/bin/env bash
cat "$OTOOL_FIXTURE"
EOF
    chmod +x "$LINKAGE_TMP/bin/otool"
  }

  cleanup_linkage_check() {
    rm -rf "$LINKAGE_TMP"
    unset LINKAGE_TMP OTOOL_FIXTURE
  }

  BeforeEach 'setup_linkage_check'
  AfterEach 'cleanup_linkage_check'

  It 'accepts a binary linked only to macOS system libraries'
    OTOOL_FIXTURE="$LINKAGE_TMP/system-libs.txt"
    export OTOOL_FIXTURE
    cat > "$OTOOL_FIXTURE" <<'EOF'
/tmp/yx:
	/usr/lib/libiconv.2.dylib
	/usr/lib/libSystem.B.dylib
EOF

    When run env PATH="$LINKAGE_TMP/bin:$PATH" "$TEST_PROJECT_DIR/bin/check-macos-linkage" "$LINKAGE_TMP/yx"
    The status should be success
    The output should include 'macOS release linkage is portable'
  End

  It 'rejects Homebrew libraries'
    OTOOL_FIXTURE="$LINKAGE_TMP/homebrew.txt"
    export OTOOL_FIXTURE
    cat > "$OTOOL_FIXTURE" <<'EOF'
/tmp/yx:
	/opt/homebrew/opt/libiconv/lib/libiconv.2.dylib
EOF

    When run env PATH="$LINKAGE_TMP/bin:$PATH" "$TEST_PROJECT_DIR/bin/check-macos-linkage" "$LINKAGE_TMP/yx"
    The status should be failure
    The error should include 'non-portable macOS release linkage'
    The error should include '/opt/homebrew/opt/libiconv/lib/libiconv.2.dylib'
  End

  It 'rejects dynamically linked OpenSSL outside Homebrew'
    OTOOL_FIXTURE="$LINKAGE_TMP/openssl.txt"
    export OTOOL_FIXTURE
    cat > "$OTOOL_FIXTURE" <<'EOF'
/tmp/yx:
	/Library/OpenSSL/lib/libssl.4.dylib
	/Library/OpenSSL/lib/libcrypto.4.dylib
EOF

    When run env PATH="$LINKAGE_TMP/bin:$PATH" "$TEST_PROJECT_DIR/bin/check-macos-linkage" "$LINKAGE_TMP/yx"
    The status should be failure
    The error should include 'non-portable macOS release linkage'
    The error should include '/Library/OpenSSL/lib/libssl.4.dylib'
  End
End
