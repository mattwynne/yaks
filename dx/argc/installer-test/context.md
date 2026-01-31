## Fix Installer Test

**The Problem:**
The installer test uses Docker (Linux) but is running on macOS, so the argc binary in the release is a macOS Mach-O binary that can't execute in Docker.

**Why This Happens:**
1. Nix build on macOS produces macOS binaries
2. The release zip includes the macOS argc binary
3. Docker runs Linux, can't execute Mach-O binaries
4. Test fails with: "Cannot run macOS (Mach-O) executable in Docker"

**Current Status:**
- Test shows "WARNED - 1" not "FAILED"
- The second test (argc not in PATH) passes
- Platform mismatch is expected in development

**Options to Fix:**

### Option 1: Skip Docker test on macOS (Easiest)
Just skip the installer test when running on macOS in development. CI can run it on Linux.

### Option 2: Cross-compile argc for Linux (Complex)
Modify flake.nix to build Linux binaries even on macOS. Nix can do this but adds complexity.

### Option 3: Rebuild test without Docker (Medium)
Rewrite installer test to work without Docker, test installation directly on the host system.

### Option 4: Accept platform-specific releases (Current)
Each platform (Linux, macOS) builds its own release with correct binaries. This is standard practice.

**Test File:**
`spec/features/install.sh`

**Done Looks Like:**
Either:
- Test passes on appropriate platforms
- OR test is marked as platform-specific and skipped appropriately
