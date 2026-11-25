# Import et export — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `on_export_json`, `on_import_json` (`src/main.rs`).
- `Book::from_json` vérifie avant d'appliquer ; `Undo::Book` garde l'ancien livre.
- Clés inconnues (réglages propres au téléphone) : ignorées, et non réécrites.
