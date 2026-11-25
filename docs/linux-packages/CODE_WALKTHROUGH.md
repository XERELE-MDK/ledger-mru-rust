# Linux packages — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md) · [Setup](INSTALL.md)

- `packaging/build_packages.sh`: one staging tree, four wrappers.
- `ledger-desktop/glibc_compat.c` and `build.rs` (`--wrap`) bring the required glibc down to **2.34**: built on a recent distribution, the program used to refuse to start elsewhere.
- The minimum glibc is measured and written into the `.deb`.
- `.github/workflows/release.yml` redoes everything on every `v*` tag.
