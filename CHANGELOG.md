# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · numbers: [SemVer](https://semver.org/).
The three repositories ([rust](https://github.com/XERELE-MDK/ledger-mru-rust), [flutter](https://github.com/XERELE-MDK/ledger-mru-flutter), [mobile](https://github.com/XERELE-MDK/ledger-mru-mobile)) move forward with **the same number**.

> Git history starts at **1.1.3**. Earlier versions are listed for the record: their code was not kept, so they have no tag and no downloads.

## [Unreleased]

### Added
- `tools/bench_desktop.py`: start-up time and memory of the desktop apps.
### Changed
- Documentation and README are now in English.

## [1.2.0] — 2026-11-25

### Added
- **Opening balances**: what you own and owe on day one, as one balanced entry ([doc](docs/opening-balances/FUNCTIONALITY.md)).
- Per-feature documentation, automated builds, GPL-3.0 licence.
### Fixed
- `1,250` is read as 1250, not 1.25: same rules as Flutter, on shared test cases.

## [1.1.3] — 2026-11-25

### Added
- "Start with an empty book": removes the demo, and nothing else.
### Fixed
- Two demos in two languages no longer conflict.

## 1.1.2 — 2026-11-25

### Fixed
- Sync loop: an identical file is no longer rewritten.
- A file that arrived late, but older than the others, went unnoticed.
- Undoing a deletion that was already synced did not hold.
- "Reset" and "Replace" are refused while sync is on.

## 1.1.1 — 2026-11-25

### Fixed
- An infinite amount (`inf`, `1e400`) made the book unreadable.
- An unreadable book is set aside, untouched; sync is disconnected.

## 1.1.0 — 2026-11-24

### Added
- New icon, Linux packages, glibc 2.34 compatibility.

[1.2.0]: https://github.com/XERELE-MDK/ledger-mru-rust/releases/tag/v1.2.0
[1.1.3]: https://github.com/XERELE-MDK/ledger-mru-rust/releases/tag/v1.1.3
