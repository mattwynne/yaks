# Changelog

All notable changes to yaks will be documented in this file.

This project uses SemVer. While yaks is in the `0.x` phase, breaking changes may happen between minor versions and will be called out here.

## [Unreleased]

### Fixed

- `yx context` and `yx field` now wait for delayed piped input instead of silently discarding writes.

## [0.5.0] - 2026-10-06

### Added

- Added `yx delete` as an alias for `yx remove`, including shell completion support.

## [0.4.0] - 2026-10-06

### Removed

- Removed the destructive `yx reset --git-from-disk` recovery mode; Git is now always authoritative over the disposable `.yaks` projection.

### Fixed

- Worktree-local `.yaks` projections now refresh when the shared yak event ref changes.

## [0.3.0] - 2026-08-12

### Added

- Added `yx events watch` and `yx events wait-for-next` for scripts and agents to observe committed domain events.

### Changed

- Readiness is now derived from workflow state, hierarchy, and blockers, with explanations in list/show/tree and JSON output.
- `yx list --ready` now emits flat yak IDs by default.
- External blocking now uses manual blockers instead of the `blocked` workflow state; legacy blocked yaks migrate to `todo` with a manual blocker.
- `bin/release` and `bin/dev release X.Y.Z` now push `main` and the release tag to the configured upstream remote after successful checks, commit, and tag creation.

### Deprecated

### Removed

### Fixed

- Long yak titles now use bounded, deterministic slugs and IDs instead of exceeding filesystem path-component limits.
- Blocker cycle validation errors now describe the problem as a `circular dependency`.

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
