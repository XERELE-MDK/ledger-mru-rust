# Appearance — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `ui/theme.slint`, `global P`: every colour is `dark ? … : …`.
- `apply_theme()` sets `P.dark` **and** `Palette.color-scheme` for the standard widgets.
- `detect_system_dark()`: Slint first, then `gsettings`.
