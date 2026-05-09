# Changelog

All notable changes to yaks will be documented in this file.

This project uses SemVer. While yaks is in the `0.x` phase, breaking changes may happen between minor versions and will be called out here.

## [Unreleased]

### Added

### Changed

- `bin/release` and `bin/dev release X.Y.Z` now push `main` and the release tag to the configured upstream remote after successful checks, commit, and tag creation.

### Deprecated

### Removed

### Fixed

### Security

## [0.2.0] - 2026-05-08

### Added

- Stable releases can now be cut from immutable `vX.Y.Z` tags, publishing GitHub Releases with packaged binaries and changelog-derived release notes.
- An `edge` prerelease channel now publishes mutable builds from `main` for users who want the newest unreleased build.
- Release validation now checks version metadata, lockfile consistency, changelog content, and `yx --version` before publishing.
- `bin/release` and `bin/dev release X.Y.Z` now prepare the release commit, annotated tag, changelog promotion, and clear push instructions.
- Added `docs/releases.md` to document version policy, stable and edge channels, changelog discipline, automation expectations, and failure recovery.

### Changed

- The installer now defaults to the latest stable release, supports `YX_VERSION` for a specific stable version, supports `YX_CHANNEL=edge` for prerelease installs, and rejects conflicting stable-version/channel choices.

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0] - YYYY-MM-DD

### Added

- Initial yaks CLI.
