# Changelog

All notable changes to BooksForge are documented in this file.

The format is based on [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(pre-1.0: minor bumps may include breaking changes).

> **For maintainers.** This file is the source of truth for the
> changelog slice attached to each GitHub Release by `release.yml`.
> Add new entries to `## [Unreleased]` as PRs land; on tag, move the
> Unreleased section under the new version heading and start a new
> empty Unreleased.
>
> Optionally, regenerate the body of `[Unreleased]` from Conventional
> Commits with `git-cliff --unreleased --strip header,footer`.

---

## [Unreleased]

### Added

- *(populate as PRs land — refer to MILESTONES.md and EXTERNAL_AUDIT_BACKLOG.md for the workstream)*

### Changed

-

### Fixed

-

### Security

- *(use this section for any change that closes a security or privacy gap; cross-reference the audit item or CVE if applicable)*

### Deprecated

-

### Removed

-

---

## Release-tagging procedure

When tagging a release, the steps are:

1. Confirm `## [Unreleased]` is up to date and accurately describes
   the changeset since the previous tag.
2. Bump the workspace version in `booksforge/Cargo.toml` (and
   `booksforge/apps/desktop/Cargo.toml` if it has its own version).
   The `release.yml` `preflight` job verifies the Cargo version
   matches the tag and fails fast otherwise.
3. Move `## [Unreleased]` content into a new section
   `## [X.Y.Z] - YYYY-MM-DD`.
4. Reset `## [Unreleased]` to empty subsections.
5. Commit with message `chore(release): vX.Y.Z`.
6. Tag: `git tag -a vX.Y.Z -m "BooksForge X.Y.Z"`.
7. Push tag: `git push origin vX.Y.Z`. This triggers `release.yml`.
8. Once the workflow completes, the GitHub Release is in **draft**.
   Spot-check artefacts (signed? notarised? SBOMs present? checksums
   match?) and publish manually.

---

[Unreleased]: https://github.com/DipurajThapa/BooksForge/compare/v0.0.1...HEAD
