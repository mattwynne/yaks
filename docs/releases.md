# Release process

This document describes the intended release process for Yaks. The goal is a
"no-think" release: the person cutting a release should only have to choose the
version number, run one command, push, and watch GitHub Actions publish the
artifacts.

The release automation is evolving. This document is the policy and UX target;
do not add manual steps here to compensate for missing automation. If a step is
required every time, it belongs in the release command or workflow.

## Version policy

Yaks uses [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`.

While Yaks is `0.x`, treat the version as follows:

- `0.MINOR.PATCH` is still pre-1.0 software: breaking changes are allowed in a
  minor release.
- Increment `MINOR` for user-visible behaviour changes, new commands, storage or
  sync format changes, and intentional breaking changes.
- Increment `PATCH` for bug fixes, documentation fixes that affect packaged
  output, installer fixes, and safe internal changes.
- Do not publish a stable release from a dirty working tree or from an unreviewed
  branch.

After `1.0.0`, follow SemVer strictly: breaking changes require a major version,
new backwards-compatible behaviour increments minor, and compatible fixes
increment patch.

## Release channels

### Stable

Stable releases are immutable Git tags named `vX.Y.Z`, for example `v0.2.0`.
Every stable tag should have a matching GitHub Release with packaged binaries,
checksums, and release notes.

Stable releases are for users who want a known version. Never move a stable tag
once it has been pushed. If a published release is wrong, cut a newer patch
release instead.

### Edge

The edge channel is a prerelease built from `main` and published as the `edge`
GitHub prerelease. It is for people who want the newest build without waiting
for the next stable tag.

The `edge` prerelease is mutable: publishing from `main` may replace its assets
and notes. It must never be treated as a stable version and should be clearly
marked as prerelease/unstable wherever it is referenced.

## Changelog discipline

Maintain a human-written changelog entry for every user-facing change before it
is released.

Use these rules:

- Keep an `Unreleased` section for changes on `main` that are not in a stable
  release yet.
- Group entries by user impact, such as `Added`, `Changed`, `Fixed`, and
  `Removed`.
- Write entries for users, not implementers. Prefer "`yx sync` now reports..."
  over "Refactored SyncPort".
- Mention migration, compatibility, or installer implications explicitly.
- Do not let the release command invent release notes from commit messages. It
  may copy or validate the changelog, but the changelog is the source of truth.
- When cutting `vX.Y.Z`, move the relevant `Unreleased` entries under a dated
  `X.Y.Z` heading and start a fresh empty `Unreleased` section.

If there are no changelog entries for the chosen version, the release command
should fail and tell the releaser what is missing.

## Target no-think stable release

The intended human workflow is:

```bash
dev release 0.2.0
git push origin main v0.2.0
```

or, if push support is built into the command:

```bash
dev release 0.2.0 --push
```

Then verify that GitHub Actions published the GitHub Release for `v0.2.0` and
that the expected assets are attached.

## What `dev release X.Y.Z` should do

The release command should be safe to run and safe to abort before pushing. It
should fail early with clear messages rather than publishing a partial release.

Expected behaviour:

1. Validate the requested version:
   - argument is exactly `X.Y.Z`, without a leading `v`;
   - version is greater than the current package version;
   - stable tag `vX.Y.Z` does not already exist locally or remotely.
2. Validate repository state:
   - running on `main`;
   - working tree is clean;
   - local `main` is up to date with `origin/main`;
   - required tools are installed.
3. Validate release readiness:
   - changelog has entries for `X.Y.Z`;
   - release notes can be derived from the changelog;
   - package metadata uses `X.Y.Z`.
4. Run the project checks expected for a release.
5. Build the release artifacts and checksums.
6. Update release files as needed, such as package version and changelog.
7. Commit the release changes with a predictable message, for example
   `Release vX.Y.Z`.
8. Create annotated tag `vX.Y.Z` on that commit.
9. Print the exact push command, or push when `--push` was supplied.

The command should not hide failures. If any validation, check, build, commit, or
tag step fails, it should stop and leave the repository in a recoverable state.

## GitHub Actions expectations

When `main` is pushed, the workflow should publish or refresh the `edge`
prerelease from that commit.

When a stable tag `vX.Y.Z` is pushed, the workflow should publish a stable GitHub
Release for that tag using the changelog entry for `X.Y.Z`. It should attach all
supported platform archives and checksums, and it should fail loudly if any
expected artifact is missing.

## Failure recovery

### The release command fails before committing

Fix the reported problem and run the command again. No cleanup should be needed
other than removing generated artifacts if the command tells you to.

### The release command commits but fails before tagging

Inspect the commit:

```bash
git show --stat HEAD
git status
```

If the commit is correct, create the tag manually or rerun the release command if
it can resume safely. If the commit is wrong and has not been pushed, reset it:

```bash
git reset --hard HEAD~1
```

Then fix the problem and rerun the release command.

### The tag exists locally but was not pushed

If the tag points at the correct release commit, push it:

```bash
git push origin main vX.Y.Z
```

If the tag points at the wrong commit and has not been pushed, delete it locally
and rerun the release:

```bash
git tag -d vX.Y.Z
```

### The tag was pushed but the GitHub Release failed

Do not move the tag. Fix the workflow or missing artifact problem, then rerun the
failed GitHub Actions job if possible. If rerunning cannot publish a correct
release, cut a new patch version.

### A stable release was published with bad assets

Do not replace history or retag. If the asset problem affects users, publish a
new patch release and explain the superseded release in the release notes.

### The `edge` prerelease failed

Because `edge` is mutable, fix the problem on `main` and let the next successful
workflow update the prerelease. If the bad edge assets are harmful, delete or
replace them in the GitHub Release while the fix is prepared.
