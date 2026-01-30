# Automated Installer Tests

## Current Status: Optimizing test performance

Tests are working but slow due to apt-get operations running on every test.

## Plan: Docker Base Image

**Problem:** apt-get update/install runs on every test execution, making tests slow.

**Solution:** Pre-built Docker image with dependencies baked in.

### Design

1. **Dockerfile** (`spec/features/Dockerfile.installer-test`):
```dockerfile
FROM ubuntu:22.04

RUN apt-get update && \
    apt-get install -y curl bash unzip git
```

2. **Test changes** (`spec/features/install.sh`):
   - Add `docker build -t yx-installer-test-base` before docker run
   - Change base image from `ubuntu:22.04` to `yx-installer-test-base`
   - Remove apt-get commands from test script
   - Remove output suppression for visibility

### Performance
- First run: Same as current (builds image)
- Subsequent runs: Fast (cached layers)
- Rebuilds only when Dockerfile changes

### Implementation Steps
1. Create Dockerfile
2. Update test to build and use the image
3. Run test to verify
