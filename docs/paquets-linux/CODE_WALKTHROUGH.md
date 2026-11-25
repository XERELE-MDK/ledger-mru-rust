# Paquets Linux — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md) · [Mise en place](INSTALL.md)

- `packaging/build_packages.sh` : une arborescence, quatre emballages.
- `ledger-desktop/glibc_compat.c` et `build.rs` (`--wrap`) ramènent la glibc exigée à **2.34** : compilé sur une distribution récente, le programme refusait de démarrer ailleurs.
- La glibc minimale est mesurée et reportée dans le `.deb`.
- `.github/workflows/release.yml` refait tout à chaque étiquette `v*`.
