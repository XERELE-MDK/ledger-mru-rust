# Apparence — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `ui/theme.slint`, `global P` : chaque couleur vaut `dark ? … : …`.
- `apply_theme()` règle `P.dark` **et** `Palette.color-scheme` des widgets standard.
- `detect_system_dark()` : Slint d'abord, puis `gsettings`.
