# Versioning

[← Documentation](README.md) · [Changelog](../CHANGELOG.md)

## Numbers

[SemVer](https://semver.org/): `MAJOR.MINOR.PATCH`.

| Change | Bump |
|---|---|
| the JSON format can no longer be read by older versions | MAJOR |
| new feature | MINOR |
| fix | PATCH |

The three repositories move forward with **the same number**. A release may bring "no change" to one of them.

## Branches and commits

- `main` is always releasable: CI must be green on it.
- One branch per topic: `feat/…`, `fix/…`, `docs/…`, merged without fast-forward.
- Messages: [Conventional Commits](https://www.conventionalcommits.org/) (`feat(opening): …`, `fix(sync): …`).

## Cutting a release

1. Bump the number in `Cargo.toml` and in the `ledger-core` dependency of `ledger-desktop/Cargo.toml`.
2. Write the `## [X.Y.Z] — date` section of `CHANGELOG.md`.
3. Check: `tools/release_notes.sh X.Y.Z` (fails if the numbers do not match).
4. Commit, then tag: `git tag -a vX.Y.Z -m "Ledger MRU X.Y.Z"` and `git push --follow-tags`.
5. CI builds and publishes `.deb`, `.rpm`, `.tar.gz`. The Arch package is added from an Arch machine.

Order across repositories: **flutter**, then **mobile** (which points to it), **rust** on its own.

## The contract between Dart and Rust

Both logics read **the same test files** (`amounts.json`, `merge_*.json`). Their hashes are in `CONTRACT.sha256`, identical in the rust and flutter repositories and checked by CI. Fixing a rule on one side = adding a case to these files, on both sides.
