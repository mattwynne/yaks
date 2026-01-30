# GitHub Action for Latest Release

## Goal
Publish yx.zip as a GitHub Release on every push to main, using a moving `latest` tag for unstable builds.

## Requirements
1. Create `.github/workflows/release.yml`
2. Trigger on push to main
3. Run `dev release` to create ./release/yx.zip
4. Publish to GitHub Releases using `latest` tag (force-update on each push)
5. Mark as prerelease
6. Attach yx.zip as a release asset

## Implementation
Use `softprops/action-gh-release@v1`:
```yaml
name: Publish Latest Release

on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: cachix/install-nix-action@v24
      - name: Build release
        run: dev release
      - name: Publish to latest
        uses: softprops/action-gh-release@v1
        with:
          tag_name: latest
          files: release/yx.zip
          prerelease: true
          body: "Unstable build from main branch"
```

## Stable URL
Once published, install.sh (and tests) can fetch from:
```
https://github.com/mattwynne/yaks/releases/download/latest/yx.zip
```

## Notes
- The `latest` tag will be force-updated on each push (yes, moving tags are gross but standard)
- Marked as prerelease to indicate instability
- Later: Add proper semantic versioning for stable releases
